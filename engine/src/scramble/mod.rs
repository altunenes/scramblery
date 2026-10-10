mod pixel;
mod types;
mod fourier;
mod block;
mod blur;
mod diffeomorphic;

pub use pixel::*;
pub use types::*;
pub use fourier::FourierScrambler;
pub use block::BlockScrambler;
pub use blur::BlurScrambler;
pub use diffeomorphic::DiffeomorphicScrambler;
use image::DynamicImage;

/// Scrambles an image. With face detection enabled, only the detected face regions are scrambled.
pub fn scramble_image(img: &DynamicImage, options: &ScrambleOptions) -> crate::Result<DynamicImage> {
    let face_opts = options.face_detection.as_ref();
    match &options.scramble_type {
        ScrambleType::Pixel => scramble_pixels(img, options),
        ScrambleType::Fourier(fourier_opts) => {
            let mut scrambler = FourierScrambler::new(
                img.width() as usize,
                img.height() as usize,
                fourier_opts.clone(),
                options.seed,
            );
            match face_opts {
                Some(f) => scrambler.scramble_with_face_detection(img, f),
                None => scrambler.scramble(img),
            }
        }
        ScrambleType::Block(block_opts) => {
            let mut scrambler = BlockScrambler::new(block_opts.clone(), options.seed);
            match face_opts {
                Some(f) => scrambler.scramble_with_face_detection(img, f),
                None => scrambler.scramble(img),
            }
        }
        ScrambleType::Blur(blur_opts) => {
            let scrambler = BlurScrambler::new(blur_opts.clone());
            match face_opts {
                Some(f) => scrambler.scramble_with_face_detection(img, f),
                None => scrambler.scramble(img),
            }
        }
        ScrambleType::Diffeomorphic(diff_opts) => {
            let mut scrambler = DiffeomorphicScrambler::new(diff_opts.clone(), options.seed);
            match face_opts {
                Some(f) => scrambler.scramble_with_face_detection(img, f),
                None => scrambler.scramble(img),
            }
        }
    }
}
