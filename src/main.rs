use std::{future::ready, sync::Arc};

use iced::{
    Element, Subscription, Task,
    futures::{
        Stream, StreamExt,
    },
    widget::{button, column, Image},
    advanced::image::Handle as ImageHandle,
};
use openh264::{decoder::{DecodedYUV, Decoder}, formats::YUVSource};
use option_lock::OptionLock;
use tello_edu::{Tello, TelloOptions, TelloVideoFrame, tello::Connected};
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Default)]
enum Screen {
    #[default]
    Start,

    Connecting,

    Connected {
        drone: Arc<Drone>,
        image: ImageHandle,
    },
}

#[derive(Default)]
struct State {
    screen: Screen,
}

#[derive(Debug, Clone)]
enum Message {
    Start,
    Connected(Arc<Drone>),
    Frame(ImageHandle),
}

#[derive(Debug)]
struct Drone {
    drone: Tello<Connected>,
    video: OptionLock<UnboundedReceiver<TelloVideoFrame>>,
}

async fn connect_drone() -> Arc<Drone> {
    let mut options = TelloOptions::default();
    let video = OptionLock::new(options.with_video());
    let drone = Tello::new()
        .wait_for_wifi()
        .await
        .unwrap()
        .connect_with(options)
        .await
        .unwrap();

    Arc::new(Drone { drone, video })
}

fn map_frame(frame: DecodedYUV<'_>) -> Message {
    let (width, height) = frame.dimensions_uv();
    let mut pixels = vec![0u8; frame.rgba8_len()];
    frame.write_rgba8(&mut pixels);
    let handle = ImageHandle::from_rgba(width as u32, height as u32, pixels);
    Message::Frame(handle)
}

fn decode_video(drone: Arc<Drone>) -> impl Stream<Item = Message> {
    let video = drone.video.try_take().unwrap();
    let mut decoder = Decoder::new().unwrap();
    tokio_stream::wrappers::UnboundedReceiverStream::new(video)
        .map(move |packet| {
            match decoder.decode(&packet.data) {
                Ok(Some(frame)) => Some(map_frame(frame)),
                Ok(None) => None,
                Err(e) => {
                    eprintln!("Failed to decode frame: {e}");
                    None
                },
            }
        })
        .filter_map(ready)
}

fn update(state: &mut State, msg: Message) -> Task<Message> {
    match msg {
        Message::Start => {
            println!("Startuyem!");
            state.screen = Screen::Connecting;
            Task::perform(connect_drone(), Message::Connected)
        }
        Message::Connected(drone) => {
            let copy = Arc::clone(&drone);
            state.screen = Screen::Connected {
                drone,
                image: ImageHandle::from_path("empty.png"),
            };
            Task::stream(decode_video(copy))
        }
        Message::Frame(frame) => {
            match &mut state.screen {
                Screen::Connected { image, .. } => *image = frame,
                _ => {},
            }
            Task::none()
        },
    }
}

fn view(state: &State) -> Element<'_, Message> {
    match &state.screen {
        Screen::Start => {
            column!["Hello Drone Man!", button("Start").on_press(Message::Start),].into()
        }
        Screen::Connecting => "Connecting to drone...".into(),
        Screen::Connected { drone, image } => Image::new(image.clone()).into(),
    }
}

fn subscription(state: &State) -> Subscription<Message> {
    todo!()
}

fn main() -> iced::Result {
    iced::application("Dronistan", update, view)
        //.subscription(subscription)
        .run()
}
