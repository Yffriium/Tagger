use iced::widget::image::Handle;
use image::DynamicImage;
use image::{GenericImageView, imageops::FilterType, imageops::crop_imm};
use std::path::PathBuf;
use tokio::sync::{mpsc, watch, watch::Sender};
use tokio_stream::Stream;

use crate::Message;

/// Loads a thumbnail and sends the message for the thumbnail using slower but
/// reliable methods. The general process is to load the full image, resize the
/// image to COVER a square of size (target_across x target_across), then crop
/// to fit the center square of that size.
async fn slow_load(
    tx: &tokio::sync::mpsc::Sender<Message>,
    index: usize,
    path: &PathBuf,
    target_across: u32,
    filter: FilterType,
) {
    // load the handle
    let img: DynamicImage = match image::open(path) {
        Ok(v) => v,
        Err(_) => return, // failed, just go to next iteration.
    };

    // get current dimensions and goal dimensions
    // goal dimensions are a box with the same aspect ratio, where w and h
    // are both at least target_across
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

    // now, resize img to this with filter alg
    let resized = img.resize(goal_w, goal_h, filter);
    let (resized_w, resized_h) = resized.dimensions();
    assert!(resized_w > 0, "Image has zero width!");
    assert!(resized_h > 0, "Image has zero height!");

    // crop image to center (target_across x target_across) rect
    let cropped = crop_imm(
        &resized,
        (resized_w - target_across) / 2,
        (resized_h - target_across) / 2,
        target_across,
        target_across,
    )
    .to_image();
    // let (cropped_w, cropped_h) = cropped.dimensions();

    let handle = Handle::from_rgba(target_across, target_across, cropped.into_raw());
    let _ = tx.send(Message::ImageLoaded(index, handle)).await;
}

/// Loads thumbnails for all the given images, if possible. Sends information
/// about the thumbnails back to the main thread.
///
/// # Return value
/// Tuple with:
/// * First value is the stream of messages that are produced
/// * Second value is the shutdown signal. Run .send(true) on it to tell thread to stop
pub fn get_async_values(
    files: Vec<(usize, PathBuf)>,
    target_across: u32,
) -> (impl Stream<Item = Message>, Sender<bool>) {
    let (tx, rx) = mpsc::channel::<Message>(10);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // decide which loading method

    tokio::spawn(async move {
        let filter = FilterType::Nearest;

        for (index, entry) in (*files).iter() {
            match shutdown_rx.has_changed() {
                Ok(v) if !v => {}
                _ => return, // end on error or change
            };

            // always slow load for now
            // later, we can use the OS cache to load thumbnails way faster.
            slow_load(&tx, *index, entry, target_across, filter).await;
        }

        // TODO is this correct to await like this?
        // await for it to be sent? idk...
        let _dont_care_if_failed = tx.send(Message::AllImagesLoaded).await;
    });

    (tokio_stream::wrappers::ReceiverStream::new(rx), shutdown_tx)
}
