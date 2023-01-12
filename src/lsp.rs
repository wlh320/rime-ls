use crate::rime::Rime;
use crate::utils;
use dashmap::DashMap;
use ropey::Rope;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

#[derive(Debug)]
pub struct Backend<'t> {
    client: Client,
    rime: Rime<'t>,
    pub documents: DashMap<String, Rope>,
}

impl<'t> Backend<'t> {
    pub fn new(
        client: Client,
        shared_data_dir: &'t str,
        user_data_dir: &'t str,
        log_dir: &'t str,
    ) -> Backend<'t> {
        // // init rime
        let rime = Rime::new(shared_data_dir, user_data_dir, log_dir);
        rime.init().unwrap();
        Backend {
            client,
            rime,
            documents: DashMap::new(),
        }
    }

    async fn on_change(&self, params: TextDocumentItem) {
        let rope = Rope::from_str(&params.text);
        self.documents.insert(params.uri.to_string(), rope);
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend<'static> {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "Server initialized")
            .await;
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: None,
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

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.documents.remove(&uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
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
            let max_candidates = 10;
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
                    sort_text: Some(c.order.to_string()),
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(range, c.text))),
                    ..Default::default()
                });
            }
            Some(ret)
        }();
        Ok(completions.map(CompletionResponse::Array))
    }
}
