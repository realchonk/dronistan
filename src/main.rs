use openh264::decoder::Decoder;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use sdl2::{event::Event, render::Texture};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tello_edu::{Tello, TelloOptions, VIDEO_HEIGHT, VIDEO_WIDTH};

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut options = TelloOptions::default();

    let mut video = options.with_video();

    let drone = Tello::new()
        .wait_for_wifi()
        .await
        .unwrap()
        .connect_with(options)
        .await
        .unwrap();

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("TELLO drone", VIDEO_WIDTH, VIDEO_HEIGHT)
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();


    drone.send("downvision 1").await.unwrap();
    drone.send("setbitrate 1").await.unwrap();

    drone.start_video().await.unwrap();

    println!("Startuyem!");

    let inspect = |r| match r {
        Ok(()) => {}
        Err(e) => println!("Received Error: {e}"),
    };

    let rendered = Arc::new(Mutex::new(None));
    let r2 = Arc::clone(&rendered);
    let mut surface = Surface::new(VIDEO_WIDTH, VIDEO_HEIGHT, PixelFormatEnum::RGB24).unwrap();

    let decode_task = async move {
        let mut decoder = Decoder::new().unwrap();
        while let Some(packet) = video.recv().await {
            match decoder.decode(&packet.data) {
                Ok(Some(frame)) => {
                    r2.lock().await.replace(frame);
                },
                Ok(None) => println!("No frame"),
                Err(e) => println!("Oh no, got decodi error: {e}"),
            }
        }
    };

    tokio::spawn(decode_task);

    let mut event_pump = sdl_context.event_pump()?;
    'running: loop {
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(kc), ..
                } => match kc {
                    Keycode::Q => break 'running,
                    Keycode::K => inspect(drone.take_off().await),
                    Keycode::L => inspect(drone.land().await),
                    Keycode::W => inspect(drone.move_forward(20).await),
                    Keycode::S => inspect(drone.move_back(20).await),
                    Keycode::A => inspect(drone.move_left(20).await),
                    Keycode::D => inspect(drone.move_right(20).await),
                    Keycode::Period => inspect(drone.move_down(20).await),
                    Keycode::Comma => inspect(drone.move_up(20).await),
                    _ => {}
                },
                Event::KeyUp {
                    keycode: Some(kc), ..
                } => match kc {
                    Keycode::W
                    | Keycode::S
                    | Keycode::A
                    | Keycode::D
                    | Keycode::Period
                    | Keycode::Comma => drone.stop().await.unwrap(),
                    _ => {}
                },
                _ => {}
            }
        }


        if let Some(mut x) = rendered.try_lock().ok() {
            if let Some(x) = x.take() {
                surface.with_lock_mut(|pixels| {
                    x.write_rgb8(pixels);
                });
            }
        }

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
    }

    Ok(())
}
