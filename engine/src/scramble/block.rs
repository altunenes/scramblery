use rand::seq::SliceRandom;

use super::types::BlockOptions;
use crate::planes::Planes;
use crate::seed;

/// Cuts the image into a grid of blocks and shuffles their positions.
///
/// When the image size is not a multiple of the block size, the right and bottom edges hold
/// smaller blocks. Blocks are only exchanged with blocks of the same size, so every pixel is
/// kept and the output has no gaps.
pub fn scramble(input: &Planes, options: &BlockOptions, rng: &mut seed::Rng) -> Planes {
    let (width, height) = (input.width, input.height);
    let bw = (options.block_size.0 as usize).clamp(1, width.max(1));
    let bh = (options.block_size.1 as usize).clamp(1, height.max(1));
    let cols = width.div_ceil(bw);
    let rows = height.div_ceil(bh);

    let size_of = |col: usize, row: usize| ((width - col * bw).min(bw), (height - row * bh).min(bh));
    let mut groups: std::collections::BTreeMap<(usize, usize), Vec<(usize, usize)>> = Default::default();
    for row in 0..rows {
        for col in 0..cols {
            groups.entry(size_of(col, row)).or_default().push((col, row));
        }
    }

    let mut output = input.clone();
    for (&(w, h), blocks) in &groups {
        let mut destinations = blocks.clone();
        destinations.shuffle(rng);
        for (&(sc, sr), &(dc, dr)) in blocks.iter().zip(&destinations) {
            let patch = input.crop(sc * bw, sr * bh, w, h);
            output.paste(&patch, dc * bw, dr * bh);
        }
    }

    if options.interpolate_edges {
        blend_edges(&mut output, bw, bh);
    }
    output
}

/// Replaces the two pixels on each side of every block boundary with their average.
fn blend_edges(image: &mut Planes, bw: usize, bh: usize) {
    let (width, height) = (image.width, image.height);
    let planes = image.color.iter_mut().chain(image.alpha.iter_mut());
    for plane in planes {
        for x in (bw..width).step_by(bw) {
            for y in 0..height {
                let (a, b) = (y * width + x - 1, y * width + x);
                let mean = 0.5 * (plane[a] + plane[b]);
                plane[a] = mean;
                plane[b] = mean;
            }
        }
        for y in (bh..height).step_by(bh) {
            for x in 0..width {
                let (a, b) = ((y - 1) * width + x, y * width + x);
                let mean = 0.5 * (plane[a] + plane[b]);
                plane[a] = mean;
                plane[b] = mean;
            }
        }
    }
}
