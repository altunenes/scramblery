# Methods

This document defines what each Scramblery operation does to an image and how the reported statistics are computed. The implementation is in `engine/src`; `engine/tests/properties.rs` tests the properties stated here.

## Image representation

Images are converted to floating point planes with values from 0 to 1: three planes (R, G, B) for colour images, one for grayscale images. Alpha is carried separately. 8-bit and 16-bit sources are written back at their original bit depth, rounding to the nearest level.

By default, processing operates on the encoded (gamma corrected) values as stored in the file. Optionally, processing runs in linear light: values are decoded with the sRGB transfer function before scrambling, and scrambling, matching and the range handling act on linear values, which are then encoded again. The transfer function is extended symmetrically to negative values, *f*(−*v*) = −*f*(*v*), so that values outside 0 to 1 produced by scrambling survive the round trip until they are clipped. In this mode, phase scrambling preserves the mean of linear light, which is proportional to the luminance on a calibrated sRGB display, and set average targets are computed in linear light too. The statistics reported by the analysis view are always computed on encoded values. The aperture is applied after encoding.

**Luminance** is Rec. 709 luma of the encoded values:

$$Y = 0.2126\,R + 0.7152\,G + 0.0722\,B$$

## Randomness and reproducibility

All random choices come from a ChaCha8 generator seeded with a 64-bit seed. ChaCha8 has a fixed output for a given seed, so the same seed and settings give the same result on every platform. When no seed is given, a random one is chosen and reported.

- **Batch:** files are processed in file name order; file *i* uses a seed derived from the base seed and *i*.
- **Video:** with *New each frame*, frame *i* uses a seed derived from the base seed and *i*; spatiotemporal volume *i* likewise.
- **Faces:** face region *i* uses a seed derived from the image seed and *i*.

Derivation uses the SplitMix64 finaliser. Saved images are accompanied by a JSON file holding the full settings and the seed used; batch runs write `scramblery_settings.json` (with the base seed) and `scramblery_report.csv` (with each file's seed and statistics).

## Values outside the range

Some operations, mainly phase scrambling and normalization, produce values outside 0 to 1. Two kinds of range handling are available:

- **Clip** clamps values to the range 0 to 1. The fraction of clipped samples is reported. Clipping changes mean and contrast where it occurs.
- **Compress contrast** scales every colour channel around its own mean, $v' = \mu_c + k\,(v - \mu_c)$, with the largest common factor $k \le 1$ that brings all values into range. Channel means are preserved exactly and contrast is reduced by $k$, which is reported.

## Scrambling methods

### Pixel

A fraction *p* (the intensity) of pixel positions is chosen at random without replacement, and the pixels at those positions are randomly permuted among themselves. All channels (and alpha) move together. Every pixel value is kept, so the histogram of every channel and of luminance is preserved exactly.

### Fourier phase

Each colour plane is transformed with a 2D discrete Fourier transform. A random phase field $\theta(\mathbf{k})$, uniform on $(-\pi, \pi]$, is drawn once per image and added to the phase of every channel:

$$X'_c(\mathbf{k}) = X_c(\mathbf{k})\, e^{\,i\,s\,\theta(\mathbf{k})}$$

where *s* is the intensity (0 to 1). The field is Hermitian symmetric, $\theta(-\mathbf{k}) = -\theta(\mathbf{k})$, so the result is real; $\theta = 0$ at the DC term and at other self-conjugate frequencies. Consequences:

- The amplitude spectrum of every channel is unchanged (exactly, with Wrap padding).
- The mean of every channel is unchanged.
- Because all channels receive the same phase offset, phase differences between channels are kept, so a grayscale image stays grayscale and no spurious colours appear.
- At *s* = 1 the phase is uniformly random; at *s* = 0 the image is unchanged.

**Frequency range.** The offset is applied only to frequencies whose radial spatial frequency $f = \sqrt{f_x^2 + f_y^2}$ lies in the chosen range (all, $f \le f_c$, $f \ge f_c$, or $f_l \le f \le f_h$). Frequencies outside the range are left intact, or set to zero if *remove frequencies outside range* is selected; the DC term is always kept. $f_x$ and $f_y$ are signed DFT frequencies in cycles per pixel, so Nyquist is 0.5.

**Units.** Cutoffs can be given in

- cycles per pixel (c/px), Nyquist 0.5;
- cycles per image width (c/img): $f_{\text{c/px}} = f_{\text{c/img}} / W$;
- cycles per degree (c/deg): $f_{\text{c/px}} = f_{\text{c/deg}} / \text{ppd}$, where ppd is the number of pixels per degree of visual angle at the viewing distance.

**Padding.**

- **Wrap** transforms the image at its own size; the image is treated as periodic.
- **Reflect** mirrors it to twice its width and height before the transform, which removes the discontinuity at the wrapped edges.
- **Mean** pads to twice the size with the plane's mean.

The output is cropped back to the original size.

**Grayscale.** When selected, luminance is processed as a single plane and the output is grayscale.

### Block

The image is divided into a grid of blocks of the chosen width and height, and block positions are shuffled. When the image size is not a multiple of the block size, the right and bottom edges hold smaller blocks; blocks are only exchanged with blocks of the same size, so every pixel is kept and the histograms are preserved exactly. *Smooth block edges* replaces the two pixels on each side of every block boundary with their mean.

### Gaussian blur

Separable convolution with a Gaussian of standard deviation σ pixels, truncated at 3σ and normalised to unit sum, with mirrored borders.

### Diffeomorphic

Follows Stojanoski & Cusack (2014) and their MATLAB implementation. A smooth displacement field is built from random cosine components,

$$d_x(x, y) = \sum_{i,j=1}^{n} a_{ij} \cos\!\left(2\pi i \tfrac{x}{W} + \phi^{(1)}_{ij}\right) \cos\!\left(2\pi j \tfrac{y}{H} + \phi^{(2)}_{ij}\right)$$

and likewise for $d_y$ with independent amplitudes and phases. Each field is scaled to unit RMS and then to (maximum distortion / number of steps). The final warp is the composition of that many small steps, each resampled bilinearly, and is applied to the image with bilinear interpolation.

## Matching after scrambling

A property of the scrambled image can be matched to a target before the range handling is applied. The properties are those of normalization (below): luminance mean and RMS contrast, the histogram of each channel, or the amplitude spectrum of each channel. Targets:

- **Original:** the image that was scrambled. Scrambled controls then share that low-level property with their intact versions; for example, histogram matching after phase scrambling restores the original histogram exactly while the phase stays scrambled.
- **Fixed:** given values of mean luminance and RMS contrast (luminance matching only).
- **Set average:** the average over a reference set of images, computed as for normalization. In batch processing the reference set is the input folder; for single images and videos it is a chosen reference folder, and each frame of a video is matched individually. When a target needs more contrast than the range 0 to 1 allows, the excess is clipped; the clipped fraction is reported for images, batches and videos.

For colour images, histogram matching acts on each channel separately; luminance, which mixes the channels, then matches closely but not exactly.

## Aperture

As the last step, the result can be shown through a centred window with a uniform surround: output = *w* · image + (1 − *w*) · surround. The window is a circle whose diameter is a fraction of the shorter image side, or an ellipse whose axes are that fraction of the image width and height; *w* follows the smoothstep rule of face masks, with the soft edge width given as a fraction of the window radius. The surround is a neutral grey at the image's mean luminance, the mean of each colour channel, or a given grey level. Matching runs before the aperture; to equate intact and scrambled stimuli as displayed, apply luminance normalization to the windowed outputs, which keeps the uniform surround uniform.

## Intensity series and exemplars

Batch processing can produce several outputs per input:

- **Exemplars** are differently seeded versions of the same image.
- **An intensity series** applies one exemplar's seed at several strengths, so that the levels differ only in strength. The varied parameter is the intensity in percent for pixel and Fourier scrambling, σ in pixels for blur, and the maximum distortion in pixels for diffeomorphic scrambling; block scrambling has none. Because the random phase field is fixed, Fourier levels lie on a continuum in phase (the offset at level *s* is *s* times the offset at 100%). Pixel levels are nested: the positions moved at a lower intensity are a subset of those moved at a higher one.

Outputs are named after their input, followed by `_sNN` for the exemplar (when there are several) and `_lLEVEL` for the series level, e.g. `face01_s02_l050.png`.

## Recipes

All settings can be saved as a JSON recipe and loaded in the application or passed to the command line tool with `--config`. The settings files written next to outputs are valid recipes, so any result can be reproduced from its own settings file.

## Video

Frames are decoded to full resolution (4:4:4) YUV at the source's bit depth and converted to RGB in floating point with the matrix (BT.601, BT.709 or BT.2020) and range (limited or full) that the video declares. Colours outside the RGB cube are kept as values outside 0 to 1 rather than clipped, and results are converted back with the same matrix and range and rounded to the nearest code value, so a frame that is left unchanged is written back with identical luma. With 4:2:0 sources, chroma is upsampled once on input.

Sources with more than 8 bits per sample are processed at their own depth and, with the lossless and high quality settings, written as 10-bit 4:4:4 HEVC; otherwise output is 8-bit H.264. These settings use software decoders, since hardware decoders may reduce frames to 8 bits; the fast setting uses hardware decoding and encoding.

The output carries the colour description that GStreamer reports for the input; when a file carries none, GStreamer assumes BT.601 for standard definition and BT.709 for high definition frames. For untagged or mistagged files, the matrix can be set explicitly (BT.601, BT.709 or BT.2020); it is then used for decoding, for encoding and in the output's colour description, with the transfer function and primaries of the same standard. Luma is unaffected by this choice; chroma, and hence the RGB values that are scrambled, depend on it.

Frames that do not depend on each other (fixed pattern, new each frame) are processed in parallel batches, and the transforms of a spatiotemporal volume are parallelised internally; follow motion processing runs frame by frame.

Each frame is scrambled as a whole and then processed further (matching, range handling) against that frame. Four temporal behaviours are available:

- **Fixed pattern** (default): every frame uses the base seed, so the same random phase field, warp field or permutation is applied to each frame. The pattern is stable over time and moving content passes through a fixed scramble.
- **New each frame:** frame *i* uses a seed derived from the base seed and *i*, producing dynamic noise.
- **Spatiotemporal:** frames are phase scrambled together. For each colour channel, a block of frames forms a volume (*x*, *y*, *t*) whose 3D Fourier transform receives one random phase field, Hermitian symmetric and zero at the DC term, scaled by the intensity: *X*′(**k**) = *X*(**k**) · e^(*i s θ*(**k**)), with **k** = (*k*ₓ, *k*ᵧ, *kₜ*). The same field is used for every channel. This preserves the 3D amplitude spectrum, i.e. both the spatial and the temporal frequency content, and each channel's mean, while destroying spatial and temporal structure. The volume is treated as periodic in space and time. Videos whose volume does not fit in the memory budget (4 GB by default, adjustable) are split into equally long volumes, each with its own seed derived from the base seed. Each volume also covers the first 8 frames of the next one; over those frames the two scrambles are crossfaded with equal-power weights, mixing the deviations from each frame's channel means with cos(π*u*/2) and sin(π*u*/2) and the means linearly, so that contrast stays constant through the transition. The number of volumes is reported. The scramble method is not used in this mode.
- **Follow motion:** backward optical flow from each frame to the previous one is estimated with SEA-RAFT. Between keyframes, the previous output is moved along the flow (bilinear backward warping). The warped result is trusted per pixel by comparing the motion-compensated previous *original* frame with the current one: below a luminance difference of 0.03 the warped pixel is used, above 0.10 a fresh scramble (fixed seed) is used, with a linear blend in between; motion pointing outside the frame is never trusted. Because this weight follows the edges of the original frame (eyes, mouth, outlines), it is smoothed with a Gaussian whose standard deviation is 3% of the shorter frame side, so that those details are not drawn into the output. Moving objects still appear as moving regions: this mode keeps motion by design and is not suited to anonymisation. Keyframes (every *N* frames) are scrambled fresh, optionally crossfaded over several frames.

**Faces in video.** Faces are detected in every frame and the scrambled frame is shown only inside the face masks (rectangle, ellipse or outline, as for images); outside them the original frame (Keep) or black (Remove) is shown. Scrambling the whole frame rather than each face region keeps the pattern stable as faces move and change size. When no face is found in a frame, the previous face mask is reused (moved along the flow in Follow motion mode) for up to 15 frames; the number of frames without a detected face is reported. In Follow motion mode, warped pixels are only used where they come from inside the previous face mask, so unscrambled content is never carried into a face.

## Face detection

Faces are located with the UltraFace detector (640 × 480 input, default confidence threshold 0.7), run on a single CPU thread so that detections are identical on every machine. Each detected box, scaled by the region size, is scrambled independently and blended into the image with a weight *w*: output = *w* · scrambled + (1 − *w*) · original. With normalised coordinates *u*, *v* in −1…1 across the box, the distance from the centre is *d* = max(|*u*|, |*v*|) for a rectangle or *d* = √(*u*² + *v*²) for the inscribed ellipse; *w* = 1 for *d* ≤ 1 − *f*, *w* = 0 for *d* ≥ 1, and *w* = *t*²(3 − 2*t*) with *t* = (1 − *d*)/*f* in between, where *f* is the soft edge fraction. For the **outline** shape, a square crop around the face box (1.5 times its larger side) is passed to the MediaPipe Face Mesh V2 landmark model (256 × 256 input), and the 36 landmarks along the face oval form a polygon, scaled about its centroid by the region size. Inside the polygon, *t* is the distance to the nearest edge divided by *f* times half the smaller side of the polygon's bounding box, and *w* follows the same curve. Both models run on a single CPU thread, so masks are identical on every machine. A soft edge removes the visible outline of the region but leaves a growing share of the original visible towards the edge. *Keep* leaves the rest of the image unchanged; *Remove* outputs only the scrambled faces on a transparent background, with *w* as the alpha. The number of faces found is reported; when none are found the image is returned unchanged.

## Statistics

| Statistic | Definition |
|---|---|
| Mean luminance | Mean of *Y* over all pixels. |
| RMS contrast | Standard deviation of *Y*. |
| Values at 0 or 1 | Fraction of pixels whose luminance is at either end of the range. |
| Clipped | Fraction of colour samples clipped by the operation. |
| Histogram | Counts of *Y* in 256 equal bins from 0 to 1. |
| Amplitude spectrum | DFT magnitude of *Y* divided by the number of pixels, averaged in rings of width $1/\min(W,H)$ cycles per pixel up to 0.5, excluding DC. |
| Spectral slope | Least-squares slope of $\log_{10}$ amplitude against $\log_{10}$ frequency over all rings. For the amplitude (not power) spectrum, natural images typically lie between −1 and −1.2. |
| SSIM | Mean structural similarity of the two luminance images (Wang et al., 2004), with an 11 × 11 Gaussian window of σ = 1.5 and constants $C_1 = 0.01^2$, $C_2 = 0.03^2$. |

## Normalization

Normalization treats all images in a folder as one set. Values outside 0 to 1 are clipped and the clipped fraction is reported per image.

- **Luminance.** Each image is mapped by $v' = (v - \mu)\,\sigma_t/\sigma + \mu_t$ on every colour channel, where μ and σ are the image's luminance mean and standard deviation and $\mu_t$, $\sigma_t$ the target (by default the averages over the set). Because the luma weights sum to one, the luminance of every image then has mean $\mu_t$ and standard deviation $\sigma_t$ (before clipping).
- **Histogram.** For each channel, the target distribution is the average over the set of the sorted pixel values (quantile averaging). Pixels of each image are ranked, with ties broken by position, and given the target value at the same quantile. This is exact histogram specification.
- **Spectrum.** For each channel, every image's amplitude spectrum is replaced by the average amplitude spectrum of the set while its phase is kept. Images must have the same size.

These follow the corresponding functions of the SHINE toolbox (Willenbockel et al., 2010).

## References

- Stojanoski, B., & Cusack, R. (2014). Time to wave good-bye to phase scrambling: Creating controlled scrambled images using diffeomorphic transformations. *Journal of Vision*, 14(12):6.
- Wang, Z., Bovik, A. C., Sheikh, H. R., & Simoncelli, E. P. (2004). Image quality assessment: From error visibility to structural similarity. *IEEE Transactions on Image Processing*, 13(4), 600–612.
- Willenbockel, V., Sadr, J., Fiset, D., Horne, G. O., Gosselin, F., & Tanaka, J. W. (2010). Controlling low-level image properties: The SHINE toolbox. *Behavior Research Methods*, 42(3), 671–684.
