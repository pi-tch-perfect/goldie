use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use futures_util::lock::Mutex;
use tokio::sync::oneshot;
use uuid::{uuid, Uuid};

use crate::lib::{
    pitch_shifter::DashPitchShifter,
    yt_downloader::{VideoProcessError, YtDownloader},
};

struct VideoDlActor {
    receiver: async_channel::Receiver<VideoDlActorMessage>,
    downloader: YtDownloader,
}

pub enum VideoDlActorMessage {
    DownloadVideo {
        yt_link: String,
        respond_to: oneshot::Sender<DownloadVideoResponse>,
    },
}

pub enum DownloadVideoResponse {
    Success { video_file_path: String },
    Fail,
}

impl VideoDlActor {
    fn new(
        receiver: async_channel::Receiver<VideoDlActorMessage>,
        video_downloader: YtDownloader,
    ) -> Self {
        VideoDlActor {
            receiver: receiver,
            downloader: video_downloader,
        }
    }

    async fn handle_message(&mut self, msg: VideoDlActorMessage) {
        println!("received video download message");

        match msg {
            VideoDlActorMessage::DownloadVideo {
                yt_link,
                respond_to,
            } => {
                let result = self.process_video(yt_link).await;
                let response = match result {
                    Ok(video_file_path) => DownloadVideoResponse::Success {
                        video_file_path: video_file_path,
                    },
                    Err(e) => {
                        eprintln!("Video processing error: {}", e);
                        DownloadVideoResponse::Fail
                    }
                };
                let _ = respond_to.send(response);
            }
        }
    }

    async fn process_video(&self, yt_link: String) -> Result<String, VideoProcessError> {
        // Download the video
        let (video_file_path, ext) = self.downloader.download(&yt_link).await?;
        println!("Download successful! Saved as: {}", video_file_path);

        // Process with pitch shifting
        let shifter = DashPitchShifter::new(
            &format!("{}.{}", video_file_path, ext),
            &format!("{}.mpd", video_file_path),
            -3..=3,
        );

        shifter.execute().map_err(|e| {
            VideoProcessError::PitchShiftError(format!("Pitch shift failed: {}", e))
        })?;

        Ok(video_file_path)
    }
}

async fn run_video_dl_actor(mut actor: VideoDlActor) {
    while let Ok(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

#[derive(Clone)]
pub struct VideoDlActorHandle {
    sender: async_channel::Sender<VideoDlActorMessage>,
}

impl VideoDlActorHandle {
    pub fn new(yt_downloader: YtDownloader) -> Self {
        let (sender, receiver) = async_channel::bounded(5);

        const NUM_CONSUMERS: u8 = 3;
        for _ in 0..NUM_CONSUMERS {
            let actor = VideoDlActor::new(receiver.clone(), yt_downloader.clone());
            tokio::spawn(run_video_dl_actor(actor));
        }

        Self { sender }
    }

    pub async fn download_video(&self, yt_link: String) -> DownloadVideoResponse {
        let (send, recv) = oneshot::channel();
        let msg = VideoDlActorMessage::DownloadVideo {
            yt_link: yt_link,
            respond_to: send,
        };

        println!("sending download video message to videodl actor");

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }
}
