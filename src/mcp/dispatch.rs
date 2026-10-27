//! Dispatch for the multiplexed `nb` tool and all first-class tools.

use anyhow::Result;
use rmcp::{ErrorData as McpError, model::CallToolResult};

use crate::nb::NoteTarget;

use super::McpServer;
use super::envelopes::{
    convert_mcp_line_edits, search_note_lines_result, show_note_lines_result,
};
use super::errors::{
    nb_error_result, outcome_result, parse_fingerprint, show_result,
    text_result, todo_shaped_add_rejection, todo_shaped_content, tool_error,
};
use super::schemas::*;

impl McpServer {
    pub(crate) async fn dispatch_nb(
        &self,
        call: NbCall,
    ) -> Result<CallToolResult, McpError> {
        macro_rules! parse_or_return {
            ($ty:ty, $value:expr) => {
                match parse_args::<$ty>($value) {
                    Ok(args) => args,
                    Err(message) => return Ok(tool_error(message)),
                }
            };
        }

        let command = call.command.trim();
        if command.is_empty() {
            return Ok(tool_error(
                "Invalid command: command must be non-empty.\n\
                 Hint: call help with query 'nb' to list available commands.",
            ));
        }

        // Strip "nb." prefix if present.
        let subcommand = command.strip_prefix("nb.").unwrap_or(command);

        let result = match subcommand {
            "status" => {
                let args: StatusArgs = parse_or_return!(StatusArgs, call.args);
                text_result(
                    self.nb
                        .show_notebook_status(args.notebook.as_deref())
                        .await,
                )
            },
            "notebooks" => text_result(self.nb.list_notebooks().await),
            "add" => {
                let args: AddArgs = parse_or_return!(AddArgs, call.args);
                if todo_shaped_content(&args.content) {
                    return Ok(todo_shaped_add_rejection());
                }
                outcome_result(
                    self.nb
                        .add_note(
                            args.title.as_deref(),
                            &args.content,
                            &args.tags,
                            args.folder.as_deref(),
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            },
            "show" => {
                let args: ShowArgs = parse_or_return!(ShowArgs, call.args);
                show_result(
                    self.nb
                        .show_note(&args.id, args.notebook.as_deref())
                        .await,
                )
            },
            "edit" => {
                return Ok(tool_error(
                    "Invalid command: edit was removed in the nb-api 0.3 cutover.\n\
                     Hint: use the direct body-aware tools replace_note_body, \
                     edit_note_substring, edit_note_lines, retitle_note, or \
                     edit_note_tags.",
                ));
            },
            "delete" => {
                let args: DeleteArgs = parse_or_return!(DeleteArgs, call.args);
                outcome_result(
                    self.nb
                        .delete_note(&args.id, args.notebook.as_deref())
                        .await,
                )
            },
            "move" => {
                let args: MoveArgs = parse_or_return!(MoveArgs, call.args);
                outcome_result(
                    self.nb
                        .move_note(
                            &args.id,
                            &args.destination,
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            },
            "list" => {
                let args: ListArgs = parse_or_return!(ListArgs, call.args);
                text_result(
                    self.nb
                        .list_notes(
                            args.folder.as_deref(),
                            &args.tags,
                            args.limit,
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            },
            "search" => {
                let args: SearchArgs = parse_or_return!(SearchArgs, call.args);
                if args.queries.is_empty() {
                    return Ok(tool_error(
                        "Invalid args for search.\n\
                         Reason: queries must be a non-empty array.\n\
                         Hint: pass queries as an array of one or more strings.",
                    ));
                }
                text_result(
                    self.nb
                        .search_notes(
                            &args.queries,
                            args.mode,
                            &args.tags,
                            args.folder.as_deref(),
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            },
            "todo" => {
                let args: TodoArgs = parse_or_return!(TodoArgs, call.args);
                outcome_result(
                    self.nb
                        .add_todo(
                            &args.title,
                            args.description.as_deref(),
                            &args.tasks,
                            &args.tags,
                            args.folder.as_deref(),
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            },
            "do" => {
                let args: TaskIdArgs = parse_or_return!(TaskIdArgs, call.args);
                outcome_result(
                    self.nb
                        .mark_task_done(
                            &args.id,
                            args.task_number,
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            },
            "undo" => {
                let args: TaskIdArgs = parse_or_return!(TaskIdArgs, call.args);
                outcome_result(
                    self.nb
                        .unmark_task_done(
                            &args.id,
                            args.task_number,
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            },
            "tasks" => {
                let args: TasksArgs = parse_or_return!(TasksArgs, call.args);
                text_result(
                    self.nb
                        .list_tasks(
                            args.folder.as_deref(),
                            args.status,
                            args.recursive,
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            },
            "bookmark" => {
                let args: BookmarkArgs =
                    parse_or_return!(BookmarkArgs, call.args);
                outcome_result(
                    self.nb
                        .add_bookmark(
                            &args.url,
                            args.title.as_deref(),
                            &args.tags,
                            args.comment.as_deref(),
                            args.folder.as_deref(),
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            },
            "folders" => {
                let args: FoldersArgs =
                    parse_or_return!(FoldersArgs, call.args);
                text_result(
                    self.nb
                        .list_folders(
                            args.parent.as_deref(),
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            },
            "mkdir" => {
                let args: MkdirArgs = parse_or_return!(MkdirArgs, call.args);
                outcome_result(
                    self.nb
                        .add_folder(&args.path, args.notebook.as_deref())
                        .await,
                )
            },
            "import" => {
                let args: ImportArgs = parse_or_return!(ImportArgs, call.args);
                text_result(
                    self.nb
                        .import_note(
                            &args.source,
                            args.folder.as_deref(),
                            args.filename.as_deref(),
                            args.convert,
                            args.notebook.as_deref(),
                        )
                        .await,
                )
            },
            "replace_note_body"
            | "edit_note_substring"
            | "edit_note_lines"
            | "retitle_note"
            | "edit_note_tags"
            | "show_note_lines"
            | "search_note_lines" => {
                return Ok(tool_error(format!(
                    "Invalid command: {subcommand} is a direct-only tool with no multiplexed alias.\n\
                     Hint: invoke the direct `{subcommand}` tool instead of the multiplexed `nb` tool."
                )));
            },
            _ => {
                return Ok(tool_error(format!(
                    "Unknown subcommand: {command}.\n\
                     Hint: call help with query 'nb' for available commands."
                )));
            },
        };

        Ok(result)
    }

    // First-class dispatch methods.
    // These reuse the same NbClient methods as dispatch_nb.

    pub(crate) async fn dispatch_add(
        &self,
        args: AddArgs,
    ) -> Result<CallToolResult, McpError> {
        if todo_shaped_content(&args.content) {
            return Ok(todo_shaped_add_rejection());
        }
        let result = self
            .nb
            .add_note(
                args.title.as_deref(),
                &args.content,
                &args.tags,
                args.folder.as_deref(),
                args.notebook.as_deref(),
            )
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_search(
        &self,
        args: SearchArgs,
    ) -> Result<CallToolResult, McpError> {
        if args.queries.is_empty() {
            return Ok(tool_error(
                "Invalid args for search.\n\
                 Reason: queries must be a non-empty array.\n\
                 Hint: pass queries as an array of one or more strings.",
            ));
        }
        let result = self
            .nb
            .search_notes(
                &args.queries,
                args.mode,
                &args.tags,
                args.folder.as_deref(),
                args.notebook.as_deref(),
            )
            .await;
        Ok(text_result(result))
    }

    pub(crate) async fn dispatch_todo(
        &self,
        args: TodoArgs,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .add_todo(
                &args.title,
                args.description.as_deref(),
                &args.tasks,
                &args.tags,
                args.folder.as_deref(),
                args.notebook.as_deref(),
            )
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_list(
        &self,
        args: ListArgs,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .list_notes(
                args.folder.as_deref(),
                &args.tags,
                args.limit,
                args.notebook.as_deref(),
            )
            .await;
        Ok(text_result(result))
    }

    pub(crate) async fn dispatch_status(
        &self,
        args: StatusArgs,
    ) -> Result<CallToolResult, McpError> {
        let result =
            self.nb.show_notebook_status(args.notebook.as_deref()).await;
        Ok(text_result(result))
    }

    pub(crate) async fn dispatch_notebooks(
        &self,
    ) -> Result<CallToolResult, McpError> {
        let result = self.nb.list_notebooks().await;
        Ok(text_result(result))
    }

    pub(crate) async fn dispatch_show(
        &self,
        args: ShowArgs,
    ) -> Result<CallToolResult, McpError> {
        let result =
            self.nb.show_note(&args.id, args.notebook.as_deref()).await;
        Ok(show_result(result))
    }

    pub(crate) async fn dispatch_replace_note_body(
        &self,
        args: ReplaceNoteBodyArgs,
    ) -> Result<CallToolResult, McpError> {
        let fingerprint = match parse_fingerprint(&args.fingerprint) {
            Ok(fp) => fp,
            Err(err) => return Ok(nb_error_result(err)),
        };
        let target = NoteTarget::selector(args.id);
        let result = self
            .nb
            .replace_note_body(
                target,
                &args.new_body,
                fingerprint,
                args.notebook.as_deref(),
            )
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_edit_note_substring(
        &self,
        args: EditNoteSubstringArgs,
    ) -> Result<CallToolResult, McpError> {
        let fingerprint = match args.fingerprint.as_deref() {
            Some(value) => match parse_fingerprint(value) {
                Ok(fp) => Some(fp),
                Err(err) => return Ok(nb_error_result(err)),
            },
            None => None,
        };
        let target = NoteTarget::selector(args.id);
        let result = self
            .nb
            .edit_note_substring(
                target,
                &args.pattern,
                &args.replacement,
                args.occurrence,
                args.expected_count,
                fingerprint,
                args.notebook.as_deref(),
            )
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_edit_note_lines(
        &self,
        args: EditNoteLinesArgs,
    ) -> Result<CallToolResult, McpError> {
        let target = NoteTarget::selector(args.id);
        let edits = convert_mcp_line_edits(args.edits);
        let result = self
            .nb
            .edit_note_lines(target, edits, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_retitle_note(
        &self,
        args: RetitleNoteArgs,
    ) -> Result<CallToolResult, McpError> {
        let target = NoteTarget::selector(args.id);
        let result = self
            .nb
            .retitle_note(target, &args.title, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_edit_note_tags(
        &self,
        args: EditNoteTagsArgs,
    ) -> Result<CallToolResult, McpError> {
        let target = NoteTarget::selector(args.id);
        let result = self
            .nb
            .edit_note_tags(
                target,
                &args.add,
                &args.remove,
                args.notebook.as_deref(),
            )
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_show_note_lines(
        &self,
        args: ShowNoteLinesArgs,
    ) -> Result<CallToolResult, McpError> {
        let target = NoteTarget::selector(args.id);
        let result = self
            .nb
            .show_note_lines(
                target,
                args.offset,
                args.limit,
                args.notebook.as_deref(),
            )
            .await;
        Ok(show_note_lines_result(result))
    }

    pub(crate) async fn dispatch_search_note_lines(
        &self,
        args: SearchNoteLinesArgs,
    ) -> Result<CallToolResult, McpError> {
        let target = NoteTarget::selector(args.id);
        let result = self
            .nb
            .search_note_lines(target, &args.pattern, args.notebook.as_deref())
            .await;
        Ok(search_note_lines_result(result))
    }

    pub(crate) async fn dispatch_delete(
        &self,
        args: DeleteArgs,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .delete_note(&args.id, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_move(
        &self,
        args: MoveArgs,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .move_note(&args.id, &args.destination, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_do(
        &self,
        args: TaskIdArgs,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .mark_task_done(
                &args.id,
                args.task_number,
                args.notebook.as_deref(),
            )
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_undo(
        &self,
        args: TaskIdArgs,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .unmark_task_done(
                &args.id,
                args.task_number,
                args.notebook.as_deref(),
            )
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_tasks(
        &self,
        args: TasksArgs,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .list_tasks(
                args.folder.as_deref(),
                args.status,
                args.recursive,
                args.notebook.as_deref(),
            )
            .await;
        Ok(text_result(result))
    }

    pub(crate) async fn dispatch_bookmark(
        &self,
        args: BookmarkArgs,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .add_bookmark(
                &args.url,
                args.title.as_deref(),
                &args.tags,
                args.comment.as_deref(),
                args.folder.as_deref(),
                args.notebook.as_deref(),
            )
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_folders(
        &self,
        args: FoldersArgs,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .list_folders(args.parent.as_deref(), args.notebook.as_deref())
            .await;
        Ok(text_result(result))
    }

    pub(crate) async fn dispatch_mkdir(
        &self,
        args: MkdirArgs,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .add_folder(&args.path, args.notebook.as_deref())
            .await;
        Ok(outcome_result(result))
    }

    pub(crate) async fn dispatch_import(
        &self,
        args: ImportArgs,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .nb
            .import_note(
                &args.source,
                args.folder.as_deref(),
                args.filename.as_deref(),
                args.convert,
                args.notebook.as_deref(),
            )
            .await;
        Ok(text_result(result))
    }
}

fn parse_args<T: serde::de::DeserializeOwned + Default>(
    value: serde_json::Value,
) -> Result<T, String> {
    if value.is_null() {
        return Ok(T::default());
    }
    let value = match value {
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                return Ok(T::default());
            }
            serde_json::Value::Object(map)
        },
        other => {
            return Err(format!(
                "Invalid args for command.\n\
                 Reason: args must be a JSON object, got {}.\n\
                 Hint: pass args as a JSON object (not a stringified JSON payload).",
                json_type_name(&other)
            ));
        },
    };

    serde_json::from_value::<T>(value).map_err(|err| {
        format!(
            "Invalid args for command.\n\
             Reason: {}.\n\
             Hint: check required fields with help query 'nb.<command>'.",
            err
        )
    })
}
fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}
