use crate::config::{Config, Settings};
use crate::consts::{trigger_ptn, NT_RE};
use crate::input::{Input, InputKind, InputResult, InputState};
use crate::rime::{Candidate, Rime, RimeResponse};
use crate::utils;
use dashmap::DashMap;
use regex::Regex;
use ropey::Rope;
use serde_json::Value;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

#[derive(Debug)]
pub struct Backend {
    client: Client,
    rime: Rime,
    documents: DashMap<String, Rope>,
    state: DashMap<String, Option<InputState>>,
    config: RwLock<Config>,
    regex: RwLock<Regex>,
}

impl Backend {
    pub fn new(client: Client) -> Backend {
        Backend {
            client,
            rime: Rime::new(),
            documents: DashMap::new(),
            state: DashMap::new(),
            config: RwLock::new(Config::default()),
            regex: RwLock::new(NT_RE.clone()),
        }
    }

    async fn init(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().await;
        let shared_data_dir = config.shared_data_dir.to_str().unwrap();
        let user_data_dir = config.user_data_dir.to_str().unwrap();
        let log_dir = config.log_dir.to_str().unwrap();
        let trigger_characters = &config.trigger_characters;
        self.compile_regex(trigger_characters).await;
        self.rime.init(shared_data_dir, user_data_dir, log_dir)
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
        let mut config = self.config.write().await;
        let settings = match serde_json::from_value::<Settings>(params) {
            Ok(s) => s,
            Err(e) => {
                self.client.show_message(MessageType::ERROR, e).await;
                return;
            }
        };
        // TODO: any better ideas?
        if let Some(v) = settings.enabled {
            config.enabled = v;
        }
        if let Some(v) = settings.max_candidates {
            config.max_candidates = v;
        }
        if let Some(v) = settings.trigger_characters {
            self.compile_regex(&v).await;
            config.trigger_characters = v;
        }
    }

    async fn notify_work_begin(&self, token: &str, message: &str) {
        // register
        let token = NumberOrString::String(String::from(token));
        self.client
            .send_request::<request::WorkDoneProgressCreate>(WorkDoneProgressCreateParams {
                token: token.clone(),
            })
            .await
            .unwrap();
        // begin
        self.client
            .send_notification::<notification::Progress>(ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                    WorkDoneProgressBegin {
                        title: message.to_string(),
                        ..Default::default()
                    },
                )),
            })
            .await;
    }

    async fn notify_work_done(&self, token: &str, message: &str) {
        let token = NumberOrString::String(String::from(token));
        self.client
            .send_notification::<notification::Progress>(ProgressParams {
                token,
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message: Some(message.to_string()),
                })),
            })
            .await;
    }

    async fn get_completions(&self, uri: Url, position: Position) -> Option<Vec<CompletionItem>> {
        let max_candidates = self.config.read().await.max_candidates;
        let is_trigger_set = !self.config.read().await.trigger_characters.is_empty();
        let re = self.regex.read().await;
        let max_len = max_candidates.to_string().len();
        let rope = self.documents.get(&uri.to_string())?;

        // get new input
        let line = Position::new(position.line, 0);
        let line_begin = utils::position_to_offset(&rope, line)?;
        let curr_char = utils::position_to_offset(&rope, position)?;
        let mut kind = InputKind::NoTrigger;
        let new_input = (curr_char <= rope.len_chars()).then(|| {
            let slice = rope.slice(line_begin..curr_char).as_str()?;
            if utils::need_to_check_trigger(is_trigger_set, slice) {
                kind = InputKind::Trigger;
                Input::from_str(&re, slice)
            } else {
                Input::from_str(&NT_RE, slice)
            }
        })??;
        let new_offset = curr_char - new_input.raw_text.len();

        // handle new input
        let mut last_state = self.state.entry(uri.to_string()).or_default();
        let InputResult { is_new, select } = match (*last_state).as_ref() {
            Some(state) => state
                .handle_new_input(&re, new_offset, &new_input, &self.rime)
                .ok()?,
            None => InputResult {
                is_new: true,
                select: None,
            },
        };

        // update state
        let session_id = if is_new {
            let bytes = new_input.pinyin.as_bytes();
            self.rime.new_session_with_keys(bytes).ok()?
        } else {
            (*last_state).as_ref().map(|s| s.session_id).unwrap()
        };
        *last_state = Some(InputState::new(
            new_input.raw_text.to_string(),
            session_id,
            new_offset,
            kind,
        ));

        // get candidates from current session
        let RimeResponse {
            preedit,
            candidates,
        } = self
            .rime
            .get_response_from_session(session_id, max_candidates)
            .ok()?;
        // prevent deleting puncts before real pinyin input
        let real_offset = new_offset
            + preedit
                .and_then(|preedit| new_input.pinyin.find(&preedit))
                .unwrap_or(0);

        // return candidates
        let range = Range::new(utils::offset_to_position(&rope, real_offset)?, position);
        let candidate_to_completion_item = |c: Candidate| -> CompletionItem {
            CompletionItem {
                label: format!("{}. {}", c.order, &c.text),
                kind: Some(CompletionItemKind::TEXT),
                detail: utils::option_string(c.comment),
                filter_text: Some(new_input.raw_text.to_string()),
                sort_text: Some(utils::order_to_sort_text(c.order, max_len)),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(range, c.text))),
                ..Default::default()
            }
        };
        let mut cand_iter = candidates.into_iter();
        select
            .and_then(|i| cand_iter.nth(i - 1)) // Note: c.order starts from 1
            .map(|c| vec![candidate_to_completion_item(c)])
            .or_else(|| Some(cand_iter.map(candidate_to_completion_item).collect()))
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "Server initialized")
            .await;

        // read user configuration
        if let Some(init_options) = params.initialization_options {
            self.init_config(init_options).await;
        } else {
            self.client
                .log_message(MessageType::INFO, "Use default config")
                .await;
        }
        // init rime
        if (self.init().await).is_err() {
            return Err(tower_lsp::jsonrpc::Error::internal_error());
        }
        // set LSP triggers
        let triggers = {
            let mut triggers = [".", ",", "-", "="].map(|x| x.to_string()).to_vec(); // pages
            let user_triggers = &self.config.read().await.trigger_characters;
            triggers.extend_from_slice(user_triggers);
            triggers
        };

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
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["toggle-rime".to_string()],
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(true),
                    },
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn shutdown(&self) -> Result<()> {
        self.rime.destroy();
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
        let mut rope = self
            .documents
            .get_mut(params.text_document.uri.as_str())
            .unwrap();
        for change in params.content_changes {
            let TextDocumentContentChangeEvent {
                range,
                range_length: _,
                text,
            } = change;
            if let Some(Range { start, end }) = range {
                let s = utils::position_to_offset(&rope, start).map(|e| e.min(rope.len_chars()));
                let e = utils::position_to_offset(&rope, end).map(|e| e.min(rope.len_chars()));
                if let (Some(s), Some(e)) = (s, e) {
                    rope.remove(s..e);
                    rope.insert(s, &text);
                }
            } else {
                // text is full content
                self.on_change(TextDocumentItem {
                    language_id: String::from("text"),
                    uri: params.text_document.uri.clone(),
                    text,
                    version: params.text_document.version,
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
        match completions {
            None => Ok(completions.map(CompletionResponse::Array)),
            Some(items) => Ok(Some(CompletionResponse::List(CompletionList {
                is_incomplete: true,
                items,
            }))),
        }
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        let command = params.command.as_ref();
        match command {
            "rime-ls.toggle-rime" => {
                self.notify_work_begin(command, command).await;
                let mut config = self.config.write().await;
                config.enabled = !config.enabled;
                let status = format!("Rime is {}", if config.enabled { "ON" } else { "OFF" });
                self.notify_work_done(command, &status).await;
                // return a bool representing if rime-ls is enabled
                return Ok(Some(Value::from(config.enabled)));
            }
            "rime-ls.sync-user-data" => {
                self.notify_work_begin(command, command).await;
                // TODO: do it in async way.
                self.rime.sync_user_data();
                self.notify_work_done(command, "Rime is Ready.").await;
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
