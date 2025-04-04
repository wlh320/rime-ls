use dashmap::DashMap;
use regex::Regex;
use ropey::Rope;
use serde_json::Value;
use std::borrow::Cow;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::config::{apply_setting, Config, Settings};
use crate::consts::{trigger_ptn, NT_RE};
use crate::input::{Input, InputResult, InputState};
use crate::rime::{Candidate, Rime, RimeError, RimeResponse};
use crate::utils::{self, Encoding};

pub struct Backend {
    client: Client,
    documents: DashMap<String, Rope>,
    state: DashMap<String, Option<InputState>>,
    config: RwLock<Config>,
    regex: RwLock<Regex>,
    encoding: RwLock<Encoding>,
}

impl Backend {
    pub fn new(client: Client) -> Backend {
        Backend {
            client,
            documents: DashMap::new(),
            state: DashMap::new(),
            config: RwLock::new(Config::default()),
            regex: RwLock::new(NT_RE.clone()),
            encoding: RwLock::new(Encoding::default()),
        }
    }

    async fn init(&self) -> std::result::Result<(), RimeError> {
        let config = self.config.read().await;
        // expand tilde
        let shared_data_dir = utils::expand_tilde(&config.shared_data_dir);
        let user_data_dir = utils::expand_tilde(&config.user_data_dir);
        let log_dir = utils::expand_tilde(&config.log_dir);
        // to str
        let shared_data_dir = shared_data_dir.to_str().unwrap();
        let user_data_dir = user_data_dir.to_str().unwrap();
        let log_dir = log_dir.to_str().unwrap();
        // compile regex
        let trigger_characters = &config.trigger_characters;
        self.compile_regex(trigger_characters).await;
        // init rime
        match Rime::init(shared_data_dir, user_data_dir, log_dir) {
            Err(RimeError::AlreadyInitialized) => {
                let info = "Use an initialized rime instance.";
                self.client.log_message(MessageType::INFO, info).await;
                Ok(())
            }
            r => r,
        }
    }

    async fn init_config(&self, params: Value) {
        let mut config = self.config.write().await;
        let new_cfg: Config = serde_json::from_value(params).unwrap_or_default();
        *config = new_cfg;
    }

    async fn compile_regex(&self, chars: &[String]) {
        if !chars.is_empty() {
            let mut regex = self.regex.write().await;
            let pattern = format!(trigger_ptn!(), chars.join(""));
            *regex = Regex::new(&pattern).unwrap();
        }
    }

    async fn apply_settings(&self, params: Value) {
        let settings = match serde_json::from_value::<Settings>(params) {
            Ok(s) => s,
            Err(e) => {
                self.client.log_message(MessageType::ERROR, &e).await;
                self.client.show_message(MessageType::ERROR, e).await;
                return;
            }
        };

        let mut config = self.config.write().await;
        apply_setting!(config <- settings.enabled);
        apply_setting!(config <- settings.max_candidates);
        apply_setting!(config <- settings.paging_characters);
        apply_setting!(config <- settings.trigger_characters, |v| {
            self.compile_regex(&v).await;
        });
        apply_setting!(config <- settings.schema_trigger_character);
        apply_setting!(config <- settings.max_tokens);
        apply_setting!(config <- settings.always_incomplete);
        apply_setting!(config <- settings.preselect_first);
        apply_setting!(config <- settings.long_filter_text);
        apply_setting!(config <- settings.show_filter_text_in_label);
        apply_setting!(config <- settings.show_order_in_label);
    }

    async fn create_work_done_progress(&self, token: NumberOrString) -> Result<NumberOrString> {
        if let Err(e) = self
            .client
            .send_request::<request::WorkDoneProgressCreate>(WorkDoneProgressCreateParams {
                token: token.clone(),
            })
            .await
        {
            self.client.log_message(MessageType::WARNING, e).await;
            return Err(tower_lsp::jsonrpc::Error::internal_error());
        }
        Ok(token)
    }

    async fn notify_work_begin(&self, token: NumberOrString, message: &str) {
        // begin
        self.client
            .send_notification::<notification::Progress>(ProgressParams {
                token,
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                    WorkDoneProgressBegin {
                        title: message.to_string(),
                        ..Default::default()
                    },
                )),
            })
            .await;
    }

    async fn notify_work_done(&self, token: NumberOrString, message: &str) {
        self.client
            .send_notification::<notification::Progress>(ProgressParams {
                token,
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message: Some(message.to_string()),
                })),
            })
            .await;
    }

    async fn get_completions(&self, uri: Url, position: Position) -> Option<CompletionList> {
        // get new input
        let rope = self.documents.get(uri.as_str())?;
        let encoding = *self.encoding.read().await;
        let line_begin = {
            let line_pos = Position::new(position.line, 0);
            utils::position_to_offset(&rope, line_pos, encoding)?
        };
        let curr_char = utils::position_to_offset(&rope, position, encoding)?;
        let new_input = {
            let re = self.regex.read().await;
            let has_trigger = !self.config.read().await.trigger_characters.is_empty();
            let schema_trigger = &self.config.read().await.schema_trigger_character;
            (curr_char <= rope.len_chars()).then(|| {
                let slice = Cow::from(rope.slice(line_begin..curr_char));
                if utils::need_to_check_trigger(has_trigger, &slice) {
                    Input::new(&re, &slice, schema_trigger)
                } else {
                    Input::new(&NT_RE, &slice, schema_trigger)
                }
            })??
        };
        let new_offset = curr_char - new_input.raw_text().len();

        // handle new input
        let mut last_state = self.state.entry(uri.into()).or_default();
        let InputResult {
            session_id,
            extra_offset,
        } = match (*last_state).as_ref() {
            Some(state) => {
                let max_tokens = self.config.read().await.max_tokens;
                state.apply_input(new_offset, &new_input, max_tokens)
            }
            None => InputState::first_input(&new_input),
        };

        // NOTE: prevent deleting puncts before real pinyin input
        //       to achieve this, puncts in rime schema should be committed directly
        let real_offset = new_offset + extra_offset;

        let start_position = utils::offset_to_position(&rope, real_offset, encoding)?;
        let range = Range::new(start_position, position);
        let filter_prefix = (self.config.read().await.long_filter_text).then_some({
            let slice = &Cow::from(rope.slice(line_begin..real_offset));
            utils::surrounding_word(slice).to_string()
        });
        // TODO: Does compiler know the right time to drop the lock,
        // or it will wait until the end of this function?
        drop(rope);

        // get candidates from current session
        let rime = Rime::global();
        let RimeResponse {
            is_incomplete,
            submitted,
            candidates,
        } = match rime.get_response_from_session(session_id) {
            Ok(r) => r,
            Err(e) => {
                self.client.log_message(MessageType::ERROR, &e).await;
                self.client.show_message(MessageType::ERROR, e).await;
                None?
            }
        };

        let is_selecting = new_input.is_selecting();
        let filter_text = filter_prefix.unwrap_or_default() + new_input.raw_text();

        // update input state
        *last_state = Some(InputState::new(
            new_input,
            session_id,
            new_offset,
            is_incomplete,
        ));
        drop(last_state);

        // convert candidates to completions
        let (show_filter_text_in_label, show_order_in_label, preselect_enabled, max_candidates) = {
            let config = self.config.read().await;
            (
                config.show_filter_text_in_label,
                config.show_order_in_label,
                config.preselect_first,
                config.max_candidates,
            )
        };
        let order_to_sort_text = utils::build_order_to_sort_text(max_candidates);
        let candidate_to_completion_item = |(i, c): (usize, Candidate)| -> CompletionItem {
            let text = match is_selecting {
                true => submitted.clone() + &c.text,
                false => c.text,
            };
            let mut label = match c.order {
                0 => text.clone(),
                _ if show_order_in_label => format!("{}. {}", c.order, &text),
                _ => text.clone(),
            };
            if show_filter_text_in_label {
                label.push_str(" (");
                label.push_str(&filter_text);
                label.push(')');
            }
            let label_details = (!c.comment.is_empty()).then_some(CompletionItemLabelDetails {
                detail: Some(c.comment.clone()),
                description: None,
            });
            CompletionItem {
                label,
                label_details,
                preselect: (preselect_enabled && i == 0).then_some(true),
                kind: Some(CompletionItemKind::TEXT),
                detail: utils::option_string(c.comment),
                filter_text: Some(filter_text.clone()),
                sort_text: Some(order_to_sort_text(c.order)),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(range, text))),
                ..Default::default()
            }
        };

        // return completions
        let is_incomplete = self.config.read().await.always_incomplete || is_incomplete;
        let item_iter = candidates
            .into_iter()
            .enumerate()
            .map(candidate_to_completion_item);

        Some(CompletionList {
            is_incomplete,
            items: item_iter.collect(),
        })
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // read user configuration
        if let Some(init_options) = params.initialization_options {
            self.init_config(init_options).await;
        } else {
            self.client
                .log_message(MessageType::INFO, "Use default config")
                .await;
        }
        // init rime
        if let Err(e) = self.init().await {
            self.client.log_message(MessageType::ERROR, &e).await;
            self.client.show_message(MessageType::ERROR, e).await;
            return Err(tower_lsp::jsonrpc::Error::internal_error());
        }
        // notify client
        self.client
            .log_message(MessageType::INFO, "Rime-ls Language Server initialized")
            .await;
        // set LSP triggers
        let triggers = {
            let mut triggers = self.config.read().await.paging_characters.clone(); // for paging
            let user_triggers = &self.config.read().await.trigger_characters;
            triggers.extend_from_slice(user_triggers);
            triggers
        };
        // negotiate position encoding
        let encoding_options = params
            .capabilities
            .general
            .and_then(|g| g.position_encodings);
        let encoding = utils::select_encoding(encoding_options);
        *self.encoding.write().await = encoding;

        // return
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "rime-ls".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                position_encoding: Some(PositionEncodingKind::new(encoding.as_str())),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(triggers),
                    ..CompletionOptions::default()
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "rime-ls.toggle-rime".to_string(),
                        "rime-ls.sync-user-data".to_string(),
                    ],
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(true),
                    },
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let url = params.text_document.uri.into();
        let rope = Rope::from(params.text_document.text);
        self.documents.insert(url, rope);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let encoding = *self.encoding.read().await;
        let url = params.text_document.uri;
        if let Some(mut rope) = self.documents.get_mut(url.as_str()) {
            for change in params.content_changes {
                let TextDocumentContentChangeEvent { range, text, .. } = change;
                match range {
                    // incremental change
                    Some(Range { start, end }) => {
                        let s = utils::position_to_offset(&rope, start, encoding);
                        let e = utils::position_to_offset(&rope, end, encoding);
                        if let (Some(s), Some(e)) = (s, e) {
                            rope.remove(s..e);
                            rope.insert(s, &text);
                        }
                    }
                    // full content change
                    None => {
                        *rope = Rope::from(text);
                    }
                }
            }
        }
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.client
            .log_message(MessageType::INFO, "settings changed")
            .await;
        self.apply_settings(params.settings).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.as_str();
        self.documents.remove(uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        if !self.config.read().await.enabled {
            return Ok(None);
        }
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let completions = self.get_completions(uri, position).await;
        Ok(completions.map(CompletionResponse::List))
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        let command: &str = params.command.as_ref();
        let token = {
            match params.work_done_progress_params.work_done_token {
                Some(token) => token,
                None => {
                    let token = NumberOrString::String(command.to_string());
                    self.create_work_done_progress(token).await?
                }
            }
        };
        match command {
            "rime-ls.toggle-rime" => {
                self.notify_work_begin(token.clone(), command).await;
                let mut config = self.config.write().await;
                config.enabled = !config.enabled;
                let status = match config.enabled {
                    true => "Rime is ON",
                    false => "Rime is OFF",
                };
                self.notify_work_done(token.clone(), status).await;
                // return a bool representing if rime-ls is enabled
                return Ok(Some(Value::from(config.enabled)));
            }
            "rime-ls.sync-user-data" => {
                self.notify_work_begin(token.clone(), command).await;
                Rime::global().sync_user_data();
                self.notify_work_done(token.clone(), "Rime is Ready.").await;
            }
            _ => {
                self.client
                    .log_message(MessageType::WARNING, "No such rime-ls command")
                    .await;
            }
        }
        Ok(None)
    }
}
