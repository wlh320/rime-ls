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
use crate::utils;

pub struct Backend {
    client: Client,
    documents: DashMap<String, Rope>,
    state: DashMap<String, Option<InputState>>,
    config: RwLock<Config>,
    regex: RwLock<Regex>,
}

impl Backend {
    pub fn new(client: Client) -> Backend {
        Backend {
            client,
            documents: DashMap::new(),
            state: DashMap::new(),
            config: RwLock::new(Config::default()),
            regex: RwLock::new(NT_RE.clone()),
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
                self.client.show_message(MessageType::INFO, info).await;
                Ok(())
            }
            r => r,
        }
    }

    async fn on_change(&self, params: TextDocumentItem) {
        let rope = Rope::from_str(&params.text);
        self.documents.insert(params.uri.to_string(), rope);
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
    }

    async fn create_work_done_progress(&self, token: NumberOrString) -> Result<NumberOrString> {
        if let Err(e) = self
            .client
            .send_request::<request::WorkDoneProgressCreate>(WorkDoneProgressCreateParams {
                token: token.clone(),
            })
            .await
        {
            self.client.show_message(MessageType::WARNING, e).await;
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
        let rope = self.documents.get(&uri.to_string())?;
        let line_begin = {
            let line_pos = Position::new(position.line, 0);
            utils::position_to_offset(&rope, line_pos)?
        };
        let curr_char = utils::position_to_offset(&rope, position)?;
        let new_input = {
            let re = self.regex.read().await;
            let has_trigger = !self.config.read().await.trigger_characters.is_empty();
            (curr_char <= rope.len_chars()).then(|| {
                let slice = Cow::from(rope.slice(line_begin..curr_char));
                if utils::need_to_check_trigger(has_trigger, &slice) {
                    Input::from_str(&re, &slice)
                } else {
                    Input::from_str(&NT_RE, &slice)
                }
            })??
        };
        let new_offset = curr_char - new_input.borrow_raw_text().len();

        // handle new input
        let mut last_state = self.state.entry(uri.to_string()).or_default();
        let InputResult {
            session_id,
            raw_input,
        } = match (*last_state).as_ref() {
            Some(state) => {
                let schema_trigger = &self.config.read().await.schema_trigger_character;
                state.handle_new_input(
                    new_offset,
                    &new_input,
                    schema_trigger,
                    self.config.read().await.max_tokens,
                )
            }
            None => InputState::handle_first_state(&new_input),
        };

        // get candidates from current session
        let rime = Rime::global();
        let RimeResponse {
            is_incomplete,
            submitted,
            candidates,
        } = match rime.get_response_from_session(session_id) {
            Ok(r) => r,
            Err(e) => {
                self.client.log_message(MessageType::ERROR, e).await;
                None?
            }
        };

        // prevent deleting puncts before real pinyin input
        let real_offset = new_offset
            + raw_input
                .and_then(|rime_raw_input| new_input.borrow_pinyin().rfind(&rime_raw_input))
                .unwrap_or(0);

        // candidates to completions
        let range = Range::new(utils::offset_to_position(&rope, real_offset)?, position);
        let filter_text = new_input.borrow_raw_text().to_string();
        let order_to_sort_text = {
            let max_candidates = self.config.read().await.max_candidates;
            utils::build_order_to_sort_text(max_candidates)
        };
        let is_selecting = new_input.is_selecting();
        let preselect_enabled = self.config.read().await.preselect_first;
        let candidate_to_completion_item = |(i, c): (usize, Candidate)| -> CompletionItem {
            let text = match is_selecting {
                true => submitted.clone() + &c.text,
                false => c.text,
            };
            let label = match c.order {
                0 => text.to_string(),
                _ => format!("{}. {}", c.order, &text),
            };
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

        // update input state
        *last_state = Some(InputState::new(
            new_input,
            session_id,
            new_offset,
            is_incomplete,
        ));
        // return completions
        let item_iter = candidates
            .into_iter()
            .enumerate()
            .map(candidate_to_completion_item);
        Some(CompletionList {
            is_incomplete: (self.config.read().await.always_incomplete || is_incomplete),
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
            self.client.log_message(MessageType::ERROR, e).await;
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
        // return
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "rime-ls".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
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
        self.on_change(TextDocumentItem {
            language_id: String::from("text"),
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
        })
        .await
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        for change in params.content_changes {
            let TextDocumentContentChangeEvent {
                range,
                range_length: _,
                text,
            } = change;
            if let Some(Range { start, end }) = range {
                let mut rope = self
                    .documents
                    .get_mut(params.text_document.uri.as_str())
                    .unwrap();
                let s = utils::position_to_offset(&rope, start);
                let e = utils::position_to_offset(&rope, end);
                if let (Some(s), Some(e)) = (s, e) {
                    rope.remove(s..e);
                    rope.insert(s, &text);
                }
            } else {
                // text is full content
                self.on_change(TextDocumentItem {
                    uri: params.text_document.uri.clone(),
                    language_id: String::from("text"),
                    version: params.text_document.version,
                    text,
                })
                .await
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
        let uri = params.text_document.uri.to_string();
        self.documents.remove(&uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        if !self.config.read().await.enabled {
            return Ok(None);
        }
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        // TODO: Is it necessary to spawn another thread?
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
                let status = format!("Rime is {}", if config.enabled { "ON" } else { "OFF" });
                self.notify_work_done(token.clone(), &status).await;
                // return a bool representing if rime-ls is enabled
                return Ok(Some(Value::from(config.enabled)));
            }
            "rime-ls.sync-user-data" => {
                self.notify_work_begin(token.clone(), command).await;
                // TODO: do it in async way.
                Rime::global().sync_user_data();
                self.notify_work_done(token.clone(), "Rime is Ready.").await;
            }
            _ => {
                self.client
                    .show_message(MessageType::WARNING, "No such rime-ls command")
                    .await;
            }
        }
        Ok(None)
    }
}
