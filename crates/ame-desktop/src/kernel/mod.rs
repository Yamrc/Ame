use crate::entity::app::CloseBehavior;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use tokio::runtime::Builder;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

#[derive(Debug, Clone)]
pub struct SongInput {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
}

#[derive(Debug, Clone)]
pub enum AppCommand {
    Navigate(String),
    SubmitSearchFromQuery,
    GenerateLoginQr,
    StopLoginQrPolling,
    EnsureGuestSession,
    RefreshLoginToken,
    SetCloseBehavior(CloseBehavior),
    OpenLibraryPlaylist(i64),
    ReplaceQueueFromPlaylist(i64),
    ReplaceQueueFromDailyTracks(Option<i64>),
    EnqueueSongAndPlay(SongInput),
    EnqueueSongOnly(SongInput),
    PlayQueueItem(i64),
    RemoveQueueItem(i64),
    ClearQueue,
    PreviousTrack,
    TogglePlay,
    NextTrack,
    CyclePlayMode,
    Quit,
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Command(AppCommand),
}

#[derive(Clone)]
pub struct KernelCommandSender {
    tx: UnboundedSender<AppCommand>,
}

impl KernelCommandSender {
    pub fn send(&self, command: AppCommand) -> bool {
        self.tx.send(command).is_ok()
    }
}

pub struct KernelRuntime {
    command_sender: KernelCommandSender,
    event_rx: Receiver<AppEvent>,
}

impl KernelRuntime {
    pub fn start() -> Self {
        let (command_tx, command_rx) = unbounded_channel::<AppCommand>();
        let (event_tx, event_rx) = channel::<AppEvent>();
        spawn_kernel_loop(command_rx, event_tx);
        Self {
            command_sender: KernelCommandSender { tx: command_tx },
            event_rx,
        }
    }

    pub fn command_sender(&self) -> KernelCommandSender {
        self.command_sender.clone()
    }

    pub fn try_recv_event(&self) -> Option<AppEvent> {
        self.event_rx.try_recv().ok()
    }
}

fn spawn_kernel_loop(mut command_rx: UnboundedReceiver<AppCommand>, event_tx: Sender<AppEvent>) {
    let _ = thread::Builder::new()
        .name("ame-kernel".to_string())
        .spawn(move || {
            let runtime = match Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(_) => return,
            };
            runtime.block_on(async move {
                while let Some(cmd) = command_rx.recv().await {
                    if matches!(cmd, AppCommand::Shutdown) {
                        break;
                    }
                    let _ = event_tx.send(AppEvent::Command(cmd));
                }
            });
        });
}
