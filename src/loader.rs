use std::{path::PathBuf};
use iced::widget::image::{Handle, allocate};
use tokio::sync::{mpsc, watch, watch::Sender};
use tokio_stream::Stream;
use image::{GenericImageView, imageops::FilterType, imageops::crop_imm};
use image::DynamicImage;

use crate::{Message};


///
/// 
/// # Return value
/// Tuple with:
/// * First value is the stream of messages that are produced
/// * Second value is the shutdown signal. Run .send(true) on it to tell thread to stop
pub fn get_async_values(files: Vec<(usize, PathBuf)>, target_across: u32) -> (impl Stream<Item = Message>, Sender<bool>) {
    let (tx, rx) = mpsc::channel::<Message>(10);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);


    tokio::spawn(async move {
        let filter = FilterType::Nearest;
        for (index, entry) in (*files).iter() {
            // load the handle
            let img: DynamicImage = match image::open(&entry) {
                Ok(v) => v,
                Err(_) => continue, // failed, just go to next iteration.
            };

            let (w, h) = img.dimensions();
            let goal_w;
            let goal_h;
            if w < h {
                goal_w = target_across;
                goal_h = (h as f32 * (target_across as f32 / w as f32)).ceil() as u32;
            } else {
                goal_h = target_across;
                goal_w = (w as f32 * (target_across as f32 / h as f32)).ceil() as u32;
            }

            println!("Goal size: {} x {}", goal_w, goal_h);

            let resized = img.resize(goal_w, goal_h, filter);
            let (resized_w, resized_h) = resized.dimensions();
            assert!(resized_w > 0, "Image has zero width!");
            assert!(resized_h > 0, "Image has zero height!");
            println!("New size: {} x {}", resized_w, resized_h);

            let cropped = crop_imm(&resized, (resized_w - target_across)/2, (resized_h - target_across)/2, target_across, target_across).to_image();
            let (cropped_w, cropped_h) = cropped.dimensions();
            println!("Cropped to {}x{}", cropped_w, cropped_h);

            let handle = Handle::from_rgba(target_across, target_across, cropped.into_raw());
            match shutdown_rx.has_changed() {
                Ok(v) if !v => {},
                _ => return // end on error or change
            };
            match tx.send(Message::ImageLoaded(*index, handle)).await {
                Ok(_) => {}, // continue, don't care
                Err(_) => return
                   
            };
        }

        // TODO is this correct to await like this?
        // await for it to be sent? idk...
        match tx.send(Message::AllImagesLoaded).await {
            Ok(_) => {},
            Err(_) => return // prob shut down or smth
        }
    });

    (tokio_stream::wrappers::ReceiverStream::new(rx), shutdown_tx)
}