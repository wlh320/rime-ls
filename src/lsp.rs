use crate::config::Config;
use crate::rime::Rime;
use crate::utils;
use dashmap::DashMap;
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
    config: RwLock<Config>,
}

impl Backend {
    pub fn new(client: Client) -> Backend {
        Backend {
            client,
            rime: Rime::new(),
            documents: DashMap::new(),
            config: RwLock::new(Config::default()),
        }
    }

    async fn init(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().await;
        let shared_data_dir = config.shared_data_dir.to_str().unwrap();
        let user_data_dir = config.user_data_dir.to_str().unwrap();
        let log_dir = config.log_dir.to_str().unwrap();
        self.rime.init(shared_data_dir, user_data_dir, log_dir)
    }

    async fn on_change(&self, params: TextDocumentItem) {
        let rope = Rope::from_str(&params.text);
        self.documents.insert(params.uri.to_string(), rope);
    }

    async fn change_config(&self, params: Value) {
        let mut config = self.config.write().await;
        let new_cfg: Config = serde_json::from_value(params).unwrap_or_default();
        *config = new_cfg;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "Server initialized")
            .await;

        // read uer configuration
        if let Some(init_options) = params.initialization_options {
            self.change_config(init_options).await;
        } else {
            self.client
                .log_message(MessageType::INFO, "Use default config")
                .await;
        }
        // init rime
        if (self.init().await).is_err() {
            return Err(tower_lsp::jsonrpc::Error::internal_error());
        }

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "rime-ls".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: self.config.read().await.trigger_characters.clone(),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
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
                let s = utils::position_to_offset(&rope, &start);
                let e = utils::position_to_offset(&rope, &end);
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
        self.change_config(params.settings).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.documents.remove(&uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let max_candidates = self.config.read().await.max_candidates;
        let max_len = max_candidates.to_string().len();
        // self.client
        //     .log_message(MessageType::ERROR, "Did Completion")
        //     .await;
        let completions = || -> Option<Vec<CompletionItem>> {
            let rope = self.documents.get(&uri.to_string())?;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
            let pinyin = (offset <= rope.len_chars()).then(|| {
                let slice = rope.slice(char..offset).as_str()?;
                utils::get_pinyin(slice)
            })??;
            // dbg!(&pinyin);
            // TODO: check trigger characters
            let cands = self
                .rime
                .get_candidates_from_keys(pinyin.clone().into_bytes(), max_candidates)
                .ok()?;

            let mut ret = Vec::with_capacity(cands.len());
            let range = Range::new(
                Position {
                    line: position.line,
                    character: position.character - pinyin.len() as u32,
                },
                position,
            );
            for c in cands {
                ret.push(CompletionItem {
                    label: c.text.clone(),
                    kind: Some(CompletionItemKind::TEXT),
                    filter_text: Some(pinyin.clone()),
                    sort_text: Some(utils::order_to_sort_text(c.order, max_len)),
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(range, c.text))),
                    ..Default::default()
                });
            }
            Some(ret)
        }();
        match completions {
            None => Ok(completions.map(CompletionResponse::Array)),
            Some(items) => Ok(Some(CompletionResponse::List(CompletionList {
                is_incomplete: true,
                items,
            }))),
        }
    }
}
