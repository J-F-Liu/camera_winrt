use image::bmp::BmpDecoder;
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::collections::HashMap;
use windows::media::capture::MediaCapture;
use windows::media::media_properties::{ImageEncodingProperties, VideoEncodingProperties};
use winrt::{import, HString};

import!(
    dependencies
        os
    types
        windows::devices::enumeration::*
        windows::media::capture::MediaCapture
        windows::storage::streams::{InMemoryRandomAccessStream,DataReader,Buffer}
);

mod fps_counter;

// Make use of any WinRT APIs as needed.
// For example, here is an example of using the Windows.Foundation.Uri class:
#[async_std::main]
async fn main() -> winrt::Result<()> {
    let (width, height) = (1280, 720);

    let cameras = find_cameras().await?;
    let bmp = create_image_encoding_properties(width as u32, height as u32)?;
    let camera = start_camera(&cameras["HD Webcam"], &bmp).await?; //USB2.0 YW500 Camera, Lena3d

    let mut buffer = vec![0_u32; width * height];
    let mut window = Window::new("Camera", width, height, WindowOptions::default())
        .unwrap_or_else(|e| panic!("{}", e));
    // window.limit_update_rate(Some(std::time::Duration::from_millis(15)));
    let mut fps_counter = fps_counter::FpsCounter::new();
    let mut image_counter: u32 = 0;

    while window.is_open() {
        let frame = capture_frame(&camera, &bmp).await?;
        fill_buffer(&mut buffer, &frame);
        let fps = fps_counter.count();
        window.set_title(&format!("Camera (FPS={:.1})", fps));

        window
            .update_with_buffer(&buffer, width, height)
            .unwrap_or_else(|e| panic!("{}", e));

        if window.is_key_down(Key::Escape) {
            break;
        }
        if window.is_key_released(Key::P) {
            std::fs::write(format!("frame{}.bmp", image_counter), frame).unwrap();
            image_counter += 1;
        }
    }
    Ok(())
}

fn fill_buffer(buffer: &mut Vec<u32>, frame: &[u8]) {
    let dynamic_image =
        image::load_from_memory_with_format(frame, image::ImageFormat::Bmp).unwrap();
    let rgb_image = dynamic_image.as_rgb8().unwrap();
    for (pixel, cell) in rgb_image.pixels().zip(buffer.iter_mut()) {
        if let [r, g, b] = pixel.0 {
            *cell = u32::from_le_bytes([b, g, r, 0]);
        }
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

async fn find_cameras() -> winrt::Result<HashMap<String, HString>> {
    use windows::devices::enumeration::{DeviceClass, DeviceInformation};

    let devices =
        DeviceInformation::find_all_async_device_class(DeviceClass::VideoCapture)?.await?;

    let mut cameras: HashMap<String, HString> = HashMap::new();
    for device in devices {
        println!("{:?}", device.name()?);
        // dbg!(MediaCapture::is_video_profile_supported(device.id()?)?);
        cameras.insert(device.name()?.into(), device.id()?);
    }
    Ok(cameras)
}

async fn start_camera(
    device_id: &HString,
    encoding: &ImageEncodingProperties,
) -> winrt::Result<MediaCapture> {
    use windows::media::capture::{MediaCaptureInitializationSettings, MediaStreamType};

    let camera = MediaCapture::new()?;
    let settings = MediaCaptureInitializationSettings::new()?;
    settings.set_video_device_id(device_id)?;
    camera.initialize_with_settings_async(settings)?.await?;
    Ok(camera)
}

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
