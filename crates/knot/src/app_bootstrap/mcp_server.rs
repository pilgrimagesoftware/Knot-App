//! Starting the MCP server for the application: the thread, the runtime and
//! the stop signal it runs under, and the status the rest of the app watches.
//! The server's own lifecycle is `knot_mcp`'s supervisor; this owns only how
//! the app starts it.

use std::path::PathBuf;
use std::sync::Arc;

use knot_mcp::ToolCatalog;
use knot_messaging::QueuedNotifier;
use parking_lot::Mutex;

use crate::mcp_status;
use crate::mcp_status::McpServerStatus;

/// Starts the MCP server under supervision, reporting the stop signal that
/// ends both and the status the rest of the application watches.
///
/// The server used to be started once here and never watched again: when
/// its serve task ended the port went quiet for the rest of the process's
/// life, and a failed bind ended the MCP subsystem for the session. The
/// supervisor owns that lifecycle now; this still owns the thread, the
/// runtime and the oneshot that stops it.
///
/// A server configuration has turned off leaves the status at
/// [`knot_mcp::ServerState::Disabled`]: nothing is bound and nothing is
/// supervised.
///
/// Takes the shared surface, not a snapshot. This runs before
/// `gpui_kit::application()`, so it cannot read the global - the bootstrap
/// builds the surface first and hands the same handle to both, which is what
/// keeps the server's settings and the windows' settings one thing. The
/// catalog it builds persists the roster hours into a session, and did so
/// from a value captured here at startup.
pub(crate) fn start_mcp_server(agents: Arc<Mutex<knot_agents::AgentStore>>,
                               settings: knot_core::SharedSettings,
                               notifier: Arc<QueuedNotifier>,
                               messages: Arc<Mutex<knot_messaging::MessageStore>>,
                               queues: crate::app_support::McpQueues,
                               shared: crate::app_support::McpShared)
                               -> (tokio::sync::oneshot::Sender<()>, McpServerStatus) {
    let crate::app_support::McpShared { subagents,
                                        credentials, } = shared;
    let (stop, stop_rx) = tokio::sync::oneshot::channel();
    let status = McpServerStatus::new();
    let thread_status = status.clone();
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(err) => {
                eprintln!("failed to start MCP server runtime: {err}");
                return;
            }
        };
        runtime.block_on(async move {
                   // Read once here: what follows is startup configuration -
                   // the enable flag, the port, the folder to watch - and the
                   // supervisor does not re-read it while it runs. The
                   // catalog gets the handle rather than this value, because
                   // what *it* does happens later.
                   let startup = settings.read();
                   if !startup.mcp_server_enabled {
                       return;
                   }

                   let (discovery, repos_rx) = knot_discovery::Discovery::new();
                   if !startup.source_base_folder.is_empty()
               && let Err(err) =
                   discovery.set_source_folder(Some(PathBuf::from(&startup.source_base_folder)))
            {
                eprintln!("failed to watch source folder: {err}");
            }

                   let catalog = Arc::new(
                knot_mcp_tools::McpToolCatalog::new(agents, repos_rx, notifier)
                    .with_subagents(Arc::clone(&subagents))
                    .with_message_store(messages)
                    .with_awaiting_input_queue(queues.awaiting_input)
                    .with_activation_queue(queues.activation)
                    .with_confirmation_queue(queues.confirmations)
                    .with_autopilot_queue(queues.autopilot)
                    .with_settings(settings),
            );
                   catalog.set_bench_agents(startup.bench_agents.clone());

                   let agents_snapshot: knot_mcp::AgentsSnapshotFn = {
                       let catalog = catalog.clone();
                       Arc::new(move || catalog.agents_snapshot())
                   };
                   let hook_handler = catalog.clone();
                   let mut supervisor =
                knot_mcp::Supervisor::new(startup.mcp_server_port,
                                          catalog as Arc<dyn ToolCatalog>,
                                          agents_snapshot).with_hook_handler(hook_handler)
                                                          .with_credentials(credentials.registry)
                                                          .with_refusals(credentials.refusals);
                   // A packaged `Knot.app` has no stderr anyone reads, so
                   // the file is the only record of what the server did.
                   // With no resolvable home directory there is nowhere to
                   // put one, and the server runs as it always has.
                   //
                   // Given to the supervisor rather than to a server: it
                   // builds a fresh one per attempt, and one writer has to
                   // span every restart for the entries either side of a
                   // failure to land in the same file.
                   if let Some(directory) = knot_core::log_dir() {
                       supervisor = supervisor.with_log(directory.join(knot_mcp::LOG_FILE_NAME));
                   }
                   // Subscribed before `run`, so no transition is missed -
                   // though a `watch` receiver would read the current value
                   // even if it were not.
                   let mirroring =
                       tokio::spawn(mcp_status::mirror_server_state(supervisor.state(),
                                                                    thread_status));
                   // Returns only on the stop signal, having released the
                   // port; everything else it retries.
                   supervisor.run(stop_rx).await;
                   mirroring.abort();
                   drop(discovery);
               });
    });
    (stop, status)
}
