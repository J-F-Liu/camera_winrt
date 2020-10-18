use std::time::Instant;

pub struct FpsCounter {
    instant: Instant,
    counter: u32,
    fps: f32,
}

impl FpsCounter {
    pub fn new() -> Self {
        FpsCounter {
            instant: Instant::now(),
            counter: 0,
            fps: 0.0,
        }
    }

    pub fn count(&mut self) -> f32 {
        self.counter += 1;
        self.fps = (self.counter as f32) / self.instant.elapsed().as_secs_f32();
        if self.counter == 30 {
            self.counter = 0;
            self.instant = Instant::now();
        }
        self.fps
    }
}
