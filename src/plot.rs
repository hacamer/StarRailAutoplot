use std::{
    thread,
    time::{Duration, Instant},
};

use crate::{automation::Automation, error::SrPlotResult, utils::get_window, xcap::Window};

use colored::Colorize;

pub type ImageFile = (&'static str, Vec<u8>);
pub type CropRatio = (f32, f32, f32, f32);

const START_IMAGE_CROP: CropRatio = (122.0 / 1920.0, 31.0 / 1080.0, 98.0 / 1920.0, 58.0 / 1080.0);
const SELECT_IMAGE_CROP: CropRatio = (
    1290.0 / 1920.0,
    442.0 / 1080.0,
    74.0 / 1920.0,
    400.0 / 1080.0,
);

// The dedicated confirm-button template is only used for 16:10 game windows.
fn is_16_10_resolution(width: u32, height: u32) -> bool {
    height > 0 && ((width as f64 / height as f64) - 1.6).abs() <= 0.005
}

fn confirm_template_scale_range(window_width: u32) -> (f64, f64) {
    // Screenshots wider than 1920px are downscaled by 1920 / window_width.
    let screenshot_scale = if window_width > 1920 {
        1920.0 / window_width as f64
    } else {
        1.0
    };
    ((screenshot_scale - 0.05).max(0.05), screenshot_scale + 0.05)
}

#[cfg(test)]
mod tests {
    use super::{confirm_template_scale_range, is_16_10_resolution};

    #[test]
    fn detects_16_10_and_exact_reference_resolution() {
        assert!(is_16_10_resolution(2560, 1600));
        assert!(is_16_10_resolution(1920, 1200));
        assert!(!is_16_10_resolution(1920, 1080));
        assert!(!is_16_10_resolution(2560, 0));
    }

    #[test]
    fn scales_confirmation_template_for_high_resolution_capture() {
        let (start, end) = confirm_template_scale_range(2560);
        assert!((start - 0.70).abs() < 1e-10);
        assert!((end - 0.80).abs() < 1e-10);

        let (start, end) = confirm_template_scale_range(1920);
        assert!((start - 0.95).abs() < 1e-10);
        assert!((end - 1.05).abs() < 1e-10);
    }
}

pub struct Plot {
    select_img: ImageFile,
    game_title_name: String,
    start_img: Vec<ImageFile>,
    game_status: GameStatus,
    auto: Automation,
}

impl Plot {
    pub fn new(game_title_name: String, select_img: ImageFile, start_img: Vec<ImageFile>) -> Self {
        Self {
            auto: Automation::new(&game_title_name),
            select_img,
            game_title_name,
            start_img,
            game_status: GameStatus::Uninitialized,
        }
    }

    pub fn run(&mut self) {
        loop {
            if let Err(e) = self.check_game_status() {
                log::error!("{}", format!("{}", e).red().bold());
            }
            thread::sleep(Duration::from_millis(500));
        }
    }

    fn check_game_status(&mut self) -> SrPlotResult<()> {
        match get_window(&self.game_title_name) {
            Some(window) if window.is_active() => {
                self.game_status.set(GameStatus::Active);
                self.autoplot(&window)?;
            }
            None => {
                self.game_status.set(GameStatus::NotFound);
            }
            _ => self.game_status.set(GameStatus::Inactive),
        }

        Ok(())
    }

    fn autoplot(&mut self, window: &Window) -> SrPlotResult<()> {
        let time = Instant::now();

        self.auto.take_screenshot(START_IMAGE_CROP.into())?;

        // 缩放大小，匹配窗口分辨率
        let scale_factor = window.width() as f64 / 1920.0;
        let scale_range = if scale_factor < 1.0 {
            Some((
                ((scale_factor - 0.05) * 10.0).round() / 10.0,
                ((scale_factor + 0.05) * 10.0).round() / 10.0,
            ))
        } else {
            None
        };

        for img in &self.start_img {
            if self.auto.find_element(img, 0.9, scale_range)?.is_some() {
                self.auto.take_screenshot(SELECT_IMAGE_CROP.into())?;
                let skip_button_found = match self.auto.find_element(
                    &self.select_img,
                    0.88, // 遇到过 0.89 匹配不上，所以降低 threshold
                    scale_range,
                )? {
                    // 有选项就点击选项
                    Some(coordinate) => {
                        self.auto.click_with_coordinate(coordinate)?;
                        true
                    }
                    // 没有识别到跳过按钮时，不进行兜底点击，避免误触。
                    None => {
                        log::debug!("未识别到跳过按钮，本轮不点击");
                        false
                    }
                };

                // 只在确实识别并点击跳过按钮后，继续点击 16:10 确认按钮。
                if skip_button_found && is_16_10_resolution(window.width(), window.height()) {
                    // 给确认弹窗留出动画时间。
                    thread::sleep(Duration::from_millis(700));
                    // 弹窗实际出现后再截取按钮区域，避免在弹窗未出现时点击固定坐标。
                    self.auto.take_screenshot(None)?;
                    let confirm_img = (
                        "confirm.png",
                        include_bytes!("../assets/confirm.png").to_vec(),
                    );
                    let confirm_scale_range = confirm_template_scale_range(window.width());
                    match self
                        .auto
                        .find_element(&confirm_img, 0.88, Some(confirm_scale_range))?
                    {
                        Some(coordinate) => {
                            log::info!("已识别确认按钮，将在按钮范围内点击");
                            self.auto.click_with_coordinate(coordinate)?;
                        }
                        None => {
                            log::warn!("未识别到确认按钮，跳过确认点击以避免误触");
                        }
                    }
                }
                break;
            }
        }
        log::debug!("执行完毕！总耗时：{}ms", time.elapsed().as_millis());
        Ok(())
    }
}

impl Default for Plot {
    fn default() -> Self {
        let image_files = vec![
            ("start.png", include_bytes!("../assets/start.png").to_vec()),
            (
                "start_ps5.png",
                include_bytes!("../assets/start_ps5.png").to_vec(),
            ),
            (
                "start_xbox.png",
                include_bytes!("../assets/start_xbox.png").to_vec(),
            ),
        ];

        let select_image = (
            "select.png",
            include_bytes!("../assets/select.png").to_vec(),
        );
        Plot::new("崩坏：星穹铁道".to_string(), select_image, image_files)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum GameStatus {
    Uninitialized,
    Active,
    Inactive,
    NotFound,
}

impl GameStatus {
    fn set(&mut self, status: GameStatus) {
        if *self != status {
            *self = status;
            self.log();
        }
    }

    fn log(&self) {
        match self {
            GameStatus::Active => log::info!("{}", "游戏窗口已激活！正在执行中……".green().bold()),
            GameStatus::Inactive => {
                log::warn!("{}", "检测到游戏窗口未激活".blue().bold())
            }
            GameStatus::NotFound => {
                log::warn!("{}", "未检测到游戏窗口，等待游戏启动……".cyan().bold())
            }
            GameStatus::Uninitialized => log::error!("{}", "未知错误".red().bold()),
        }
    }
}
