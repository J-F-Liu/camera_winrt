#![windows_subsystem = "windows"]

use minifb::{Key, Menu, Window, WindowOptions};
use std::collections::HashMap;
use std::iter::FromIterator;
use std::process::Command;
use windows::media::capture::{LowLagPhotoCapture, MediaCapture};
use windows::media::capture::{MediaCaptureInitializationSettings, MediaStreamType};
use windows::media::media_properties::{
    ImageEncodingProperties, MediaRatio, VideoEncodingProperties,
};
use winrt::{import, ComInterface, HString};

// Make use of any WinRT APIs as needed.
import!(
    dependencies
        os
    types
        windows::devices::enumeration::*
        windows::media::capture::MediaCapture
        windows::storage::streams::{InMemoryRandomAccessStream,DataReader,Buffer}
);

mod fps_counter;
mod id_manager;
use id_manager::IdManager;

#[async_std::main]
async fn main() -> winrt::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let excutable = &args[0];
    let camera_name = if args.len() > 1 { &args[1] } else { "" };

    let (width, height) = (1280, 720);
    let bmp = create_image_encoding_properties(width as u32, height as u32)?;

    let cameras = find_cameras().await?;
    let camera = MediaCapture::new()?;
    let formats = start_camera(&camera, &cameras, camera_name).await?;
    let capture = camera.prepare_low_lag_photo_capture_async(&bmp)?.await?;

    let mut buffer = vec![0_u32; width * height];
    let mut window = Window::new("Camera", width, height, WindowOptions::default())
        .unwrap_or_else(|e| panic!("{}", e));
    window.limit_update_rate(Some(std::time::Duration::from_millis(15)));

    let mut menu_ids = IdManager::new();
    create_menu(&mut window, &mut menu_ids, &cameras, &formats);

    let mut fps_counter = fps_counter::FpsCounter::new();
    let mut image_counter: u32 = 0;

    while window.is_open() {
        if let Some(item_id) = window.is_menu_pressed() {
            let name = menu_ids.get(item_id);
            println!("{}", name);
            if cameras.contains_key(name) {
                // media_capture.close()?;

                // let settings = MediaCaptureInitializationSettings::new()?;
                // settings.set_video_device_id(&cameras[name])?;

                // media_capture = MediaCapture::new()?;
                // media_capture
                //     .initialize_with_settings_async(settings)?
                //     .await?;
                // std::thread::sleep(std::time::Duration::from_secs(3));
                Command::new(excutable)
                    .arg(name)
                    .spawn()
                    .expect("Open camera");
                std::process::exit(0);
            }
            if formats.contains_key(name) {
                let controller = camera.video_device_controller()?;
                controller
                    .set_media_stream_properties_async(
                        windows::media::capture::MediaStreamType::VideoRecord,
                        &formats[name],
                    )?
                    .await?;
            }
        }

        // let frame = capture_frame(&camera, &bmp).await?;
        let frame = capture_lowlag_frame(&capture).await?;
        fill_buffer(&mut buffer, &frame);

        if window.is_key_released(Key::P) {
            let filename = format!("frame{}.bmp", image_counter);
            std::fs::write(&filename, frame).unwrap();
            println!("Captured to {}", filename);
            image_counter += 1;
        }

        if window.is_key_down(Key::Escape) {
            break;
        }

        let fps = fps_counter.count();
        window.set_title(&format!("Camera (FPS={:.1})", fps));

        window
            .update_with_buffer(&buffer, width, height)
            .unwrap_or_else(|e| panic!("{}", e));
    }
    Ok(())
}

fn create_menu(
    window: &mut Window,
    menu_ids: &mut IdManager,
    cameras: &HashMap<String, HString>,
    formats: &HashMap<String, VideoEncodingProperties>,
) {
    let mut menu_cameras = Menu::new("Cameras").unwrap();
    for name in cameras.keys() {
        let id = menu_ids.add(name);
        menu_cameras.add_item(name, id).build();
    }
    window.add_menu(&menu_cameras);

    let mut menu_formats = Menu::new("Video Formats").unwrap();
    let mut video_formats = Vec::from_iter(formats.keys());
    video_formats.sort();
    for name in video_formats {
        let id = menu_ids.add(name);
        menu_formats.add_item(name, id).build();
    }
    window.add_menu(&menu_formats);
}

fn fill_buffer(buffer: &mut Vec<u32>, frame: &[u8]) {
    let dynamic_image =
        image::load_from_memory_with_format(frame, image::ImageFormat::Bmp).unwrap();
    let rgb_image = dynamic_image.as_rgb8().unwrap();
    for (pixel, cell) in rgb_image.pixels().zip(buffer.iter_mut()) {
        let [r, g, b] = pixel.0;
        *cell = u32::from_le_bytes([b, g, r, 0]);
    }
}

fn create_image_encoding_properties(
    width: u32,
    height: u32,
) -> winrt::Result<ImageEncodingProperties> {
    let bmp = ImageEncodingProperties::create_bmp()?;
    bmp.set_width(width)?;
    bmp.set_height(height)?;
    Ok(bmp)
}

fn compute_ratio(ratio: MediaRatio) -> winrt::Result<f32> {
    Ok(ratio.numerator()? as f32 / ratio.denominator()? as f32)
}

async fn find_cameras() -> winrt::Result<HashMap<String, HString>> {
    use windows::devices::enumeration::{DeviceClass, DeviceInformation};

    let devices =
        DeviceInformation::find_all_async_device_class(DeviceClass::VideoCapture)?.await?;

    let mut cameras: HashMap<String, HString> = HashMap::new();
    for device in devices {
        // println!("{:?}", device.name()?);
        // dbg!(MediaCapture::is_video_profile_supported(device.id()?)?);
        cameras.insert(device.name()?.into(), device.id()?);
    }
    Ok(cameras)
}

async fn start_camera(
    camera: &MediaCapture,
    cameras: &HashMap<String, HString>,
    camera_name: &str,
) -> winrt::Result<HashMap<String, VideoEncodingProperties>> {
    if cameras.contains_key(camera_name) {
        println!("Start camera: {}", camera_name);
        let device_id = &cameras[camera_name];
        let settings = MediaCaptureInitializationSettings::new()?;
        settings.set_video_device_id(device_id)?;
        camera.initialize_with_settings_async(settings)?.await?;
    } else {
        println!("Start default camera");
        camera.initialize_async()?.await?;
    }

    let mut formats = HashMap::new();
    let controller = camera.video_device_controller()?;
    for prop in controller.get_available_media_stream_properties(MediaStreamType::VideoRecord)? {
        let video_prop = prop.query::<VideoEncodingProperties>();
        let name = format!(
            "{}-{}: {}x{}@{}fps",
            prop.r#type()?,
            prop.subtype()?,
            video_prop.width()?,
            video_prop.height()?,
            compute_ratio(video_prop.frame_rate()?)?
        );
        // println!("{}", &name);
        formats.insert(name, video_prop);
    }

    Ok(formats)
}

#[allow(dead_code)]
async fn capture_frame(
    camera: &MediaCapture,
    encoding: &ImageEncodingProperties,
) -> winrt::Result<Vec<u8>> {
    use windows::storage::streams::{
        Buffer, DataReader, InMemoryRandomAccessStream, InputStreamOptions,
    };

    let stream = InMemoryRandomAccessStream::new()?;
    camera
        .capture_photo_to_stream_async(encoding, &stream)?
        .await?;
    stream.seek(0)?;

    let buffer = Buffer::create(stream.size()? as u32)?;
    let buffer = stream
        .read_async(buffer, stream.size()? as u32, InputStreamOptions::None)?
        .await?;
    // println!("{:?}", buffer.length());
    let mut data = vec![0u8; buffer.length()? as usize];
    let reader = DataReader::from_buffer(buffer)?;
    reader.read_bytes(&mut data)?;
    Ok(data)
}

#[allow(dead_code)]
async fn capture_lowlag_frame(capture: &LowLagPhotoCapture) -> winrt::Result<Vec<u8>> {
    use windows::storage::streams::{Buffer, DataReader, InputStreamOptions};

    let photo = capture.capture_async()?.await?;
    let frame = photo.frame()?;

    let buffer = Buffer::create(frame.size()? as u32)?;
    let buffer = frame
        .read_async(buffer, frame.size()? as u32, InputStreamOptions::None)?
        .await?;
    // println!("{:?}", buffer.length());
    let mut data = vec![0u8; buffer.length()? as usize];
    let reader = DataReader::from_buffer(buffer)?;
    reader.read_bytes(&mut data)?;
    Ok(data)
}
