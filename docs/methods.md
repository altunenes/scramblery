# Methods

What each Scramblery operation does to an image, what it keeps, and how the reported statistics are computed. The code is in `engine/src`, and `engine/tests/properties.rs` tests the properties stated here.

## Images

Images are processed as floating point planes from 0 to 1: R, G and B for colour, one plane for grayscale, with alpha carried separately. 8-bit and 16-bit files are written back at their own bit depth, rounded to the nearest level.

Processing works on the encoded (gamma corrected) values stored in the file. With **linear light**, values are decoded with the sRGB transfer function first, extended to negative values as *f*(−*v*) = −*f*(*v*) so that out-of-range values survive, and encoded again at the end. Phase scrambling then preserves the mean of linear light, which is proportional to luminance on a calibrated sRGB display, and set averages are computed in linear light too. Reported statistics always use encoded values, and the aperture is applied after encoding.

**Luminance** is Rec. 709 luma of the encoded values:

$$Y = 0.2126\,R + 0.7152\,G + 0.0722\,B$$

## Seeds and reproducibility

All randomness comes from a ChaCha8 generator with a 64-bit seed, which gives the same output on every platform. Without a seed, a random one is chosen and reported. Derived seeds use the SplitMix64 finaliser:

- **Batch:** file *i* (in file name order) uses a seed derived from the base seed and *i*.
- **Video:** with *New each frame*, frame *i* uses a seed derived from the base seed and *i*; spatiotemporal volume *i* likewise.
- **Faces:** face region *i* uses a seed derived from the image seed and *i*.

Every saved image gets a JSON file with the full settings and seed. Batch runs write `scramblery_settings.json` (with the base seed) and `scramblery_report.csv` (each file's seed and statistics).

## Values outside 0 to 1

Phase scrambling and normalization can produce values outside the range. Two options:

- **Clip** clamps to 0 to 1 and reports the clipped fraction. Clipping changes mean and contrast where it happens.
- **Compress contrast** scales each channel around its own mean, $v' = \mu_c + k\,(v - \mu_c)$, with the largest common $k \le 1$ that fits all values. Channel means stay exact; contrast drops by $k$, which is reported.

## Scrambling methods

### Pixel

A fraction *p* (the intensity) of pixel positions is chosen at random and those pixels are shuffled among themselves, all channels and alpha together.

- Every pixel value is kept, so the histograms of every channel and of luminance are exact.

### Fourier phase

The classic phase scramble: the amplitude spectrum stays, the phase is randomised. Each colour plane gets a 2D Fourier transform, and one random phase field $\theta(\mathbf{k})$, uniform on $(-\pi, \pi]$, is added to every channel:

$$X'_c(\mathbf{k}) = X_c(\mathbf{k})\, e^{\,i\,s\,\theta(\mathbf{k})}$$

with *s* the intensity (0 to 1). The field is Hermitian symmetric, $\theta(-\mathbf{k}) = -\theta(\mathbf{k})$, so the result is real, and it is 0 at the DC term and other self-conjugate frequencies.

- The amplitude spectrum of every channel is kept (exactly with Wrap padding), and so is every channel's mean.
- All channels get the same offsets, so phase differences between channels are kept: grayscale stays grayscale and no false colours appear.
- At *s* = 1 the phase is uniformly random; at *s* = 0 the image is unchanged.

**Frequency range.** Only frequencies with radial frequency $f = \sqrt{f_x^2 + f_y^2}$ in the chosen range are scrambled (all, $f \le f_c$, $f \ge f_c$, or $f_l \le f \le f_h$). The rest are kept, or set to zero with *remove frequencies outside range*; DC is always kept. Cutoffs can be given in cycles per pixel (Nyquist 0.5), cycles per image width ($f_{\text{c/px}} = f_{\text{c/img}} / W$), or cycles per degree ($f_{\text{c/px}} = f_{\text{c/deg}} / \text{ppd}$, with ppd the pixels per degree of visual angle).

**Padding.** *Wrap* transforms the image at its own size, treating it as periodic. *Reflect* mirrors it to twice its width and height, which removes the jump at the wrapped edges. *Mean* pads to twice the size with the plane's mean. The output is cropped back.

**Grayscale.** Luminance is processed as a single plane and the output is grayscale.

### Local phase

Fourier phase scrambling keeps the global spectrum but spreads contrast evenly over the image. Local phase scrambling keeps *where* contrast is, at each scale and orientation, and randomises only its local arrangement: textured regions stay textured, plain regions stay plain, and the coarse layout stays.

Each plane is split by a complex steerable filter bank (similar to the steerable pyramid of Portilla & Simoncelli, 2000) into *S* scales × *M* orientations. The bands are oriented and band-pass like Gabor or log-Gabor filters, but designed to invert exactly. Band (*s*, *o*) has the filter $G_{s,o}(f, \theta) = R_s(f)\,A_o(\theta)$:

- **Radial window $R_s$.** A raised cosine in octaves. With $u = \log_2(f_0 / f)$ and $f_0 = 1/4$ cycle per pixel, scale 0 is 1 for $u \le 0$ (so it also covers all higher frequencies). For $s \le u < s + 1$, $R_s = \cos(\tfrac{\pi}{2} h)$ and $R_{s+1} = \sin(\tfrac{\pi}{2} h)$, with $h = t^2(3 - 2t)$ and $t = u - s$. Below the coarsest scale a low-pass residual *L* takes over (1 for $u \ge S$). The squared windows sum to 1 at every frequency.
- **Angular window $A_o$.** $c \cos^{M-1}(\theta - \theta_o)$ within $\pi/2$ of $\theta_o = \pi o / M$ and 0 elsewhere, with *c* such that $\sum_o \left(A_o(\theta)^2 + A_o(\theta + \pi)^2\right) = 1$. Each filter covers half the frequency plane, so each band response is complex:

$$b_{s,o} = \mathcal{F}^{-1}\left(X \cdot G_{s,o}\right)$$

Its modulus $|b|$ is the local amplitude and its angle the local phase. Scrambling rotates every coefficient by a random offset and keeps the modulus:

$$b' = b\, e^{\,i\,s\,\varphi_{s,o}(\mathbf{x})}$$

Each band has its own offset field: Gaussian white noise, low-pass filtered with a Gaussian of standard deviation half the band's centre frequency ($f_0 / 2^s$), scaled to unit variance and mapped to uniform on $(-\pi, \pi)$ by $\varphi = \pi\,\mathrm{erf}(g/\sqrt{2})$. The offsets therefore vary smoothly over about one wavelength. All channels share the fields. The image is rebuilt with the same filters plus $X L^2$; since rotating phases pushes some energy outside a band's filter, each band is rescaled to its original energy.

- At *s* = 0 the image is returned exactly.
- The low-pass residual (the mean and everything coarser than about $4 \cdot 2^{S-1}$ pixels) is kept. Scales are in pixels, so a larger image keeps more of its structure at the same setting.
- Each band's energy is kept, so contrast at each scale and orientation is unchanged overall and stays near where it was.
- Channel means are exact with Wrap padding and close otherwise; grayscale stays grayscale.
- Unlike Fourier scrambling, the amplitude spectrum is not kept exactly.

**Padding.** *Wrap* uses the image at its own size. *Reflect* and *Mean* add margins of two coarsest wavelengths ($2^{S+2}$ pixels, at most the image size), mirrored or mean-filled, and round the size up to a product of 2, 3 and 5. Scale *s* runs on a grid $2^s$ times smaller, which is exact because it has nothing above $0.5 / 2^s$ cycles per pixel.

### Block

The image is cut into blocks of the chosen size and the blocks are shuffled. If the size is not a multiple of the block size, the right and bottom edges hold smaller blocks, which only swap with blocks of the same size.

- Every pixel is kept, so histograms are exact.
- *Smooth block edges* replaces the two pixels on each side of every block boundary with their mean.

### Gaussian blur

Separable convolution with a Gaussian of standard deviation σ pixels, truncated at 3σ, normalised to unit sum, with mirrored borders.

### Diffeomorphic

Follows Stojanoski & Cusack (2014) and their MATLAB code: the image is bent by a smooth random warp, so it stays a continuous image but loses its recognisable shape. The displacement field is a sum of random cosines,

$$d_x(x, y) = \sum_{i,j=1}^{n} a_{ij} \cos\!\left(2\pi i \tfrac{x}{W} + \phi^{(1)}_{ij}\right) \cos\!\left(2\pi j \tfrac{y}{H} + \phi^{(2)}_{ij}\right)$$

and likewise $d_y$ with its own amplitudes and phases. Each field is scaled to unit RMS, then to (maximum distortion / number of steps). The final warp composes that many small steps, each resampled bilinearly, and is applied with bilinear interpolation.

## Matching after scrambling

A property of the scrambled image can be matched to a target before range handling: luminance mean and RMS contrast, each channel's histogram, or each channel's amplitude spectrum (as in Normalization below). Targets:

- **Original:** the image that was scrambled, so controls share that property with their intact versions. For example, histogram matching after phase scrambling restores the original histogram exactly while the phase stays scrambled.
- **Fixed:** given mean luminance and RMS contrast (luminance only).
- **Set average:** the average over a reference set, computed as for normalization: the input folder in batch processing, a chosen folder for single images and videos. Video frames are matched one by one.

Contrast beyond the 0 to 1 range is clipped and reported. For colour images, histogram matching works per channel, so luminance matches closely but not exactly.

## Aperture

The last step can show the result through a window with a uniform surround: output = *w* · image + (1 − *w*) · surround, with *w* following the smoothstep rule of face masks (see Face detection) and the soft edge given as a fraction of the window radius.

- **Centred on the image:** a circle whose diameter is a fraction of the shorter side, or an ellipse with that fraction of the width and height.
- **Centred on faces:** one window per detected face, sized relative to the face box (1 passes through its corners); with several windows, *w* is the largest weight. Where no face is found only the surround is shown.

The surround is neutral grey at the image's mean luminance, each channel's mean, or a given grey. Matching runs before the aperture; to equate intact and scrambled stimuli as displayed, normalize luminance of the windowed outputs, which keeps the surround uniform.

## Intensity series and exemplars

Batch processing can make several outputs per input:

- **Exemplars:** differently seeded versions of the same image.
- **Intensity series:** one exemplar's seed at several strengths, so levels differ only in strength. The varied parameter is the intensity in percent for pixel, Fourier and local phase, σ in pixels for blur, and maximum distortion in pixels for diffeomorphic; block has none. Fourier levels lie on a continuum (the offset at level *s* is *s* times the offset at 100%), and pixel levels are nested (positions moved at a lower level are a subset of those at a higher one).

Outputs are named after the input plus `_sNN` for the exemplar (when there are several) and `_lLEVEL` for the level, e.g. `face01_s02_l050.png`.

## Recipes

All settings can be saved as a JSON recipe, loaded in the app, or passed to the command line tool with `--config`. The settings file written next to each output is itself a recipe, so any result can be reproduced from it.

## Video

**Colour.** Frames are decoded to full resolution (4:4:4) YUV at the source's bit depth and converted to float RGB with the video's own matrix (BT.601, BT.709 or BT.2020) and range (limited or full). Out-of-gamut colours stay as values outside 0 to 1, and results go back through the same matrix and range, so an unchanged frame keeps identical luma. 4:2:0 chroma is upsampled once on input. If a file's colour tag is missing or wrong, the matrix can be set by hand; it then drives decoding, encoding and the output's tag. Untagged files are read by GStreamer as BT.601 (SD) or BT.709 (HD).

**Output.** Sources above 8 bits are processed at their own depth and, with the lossless and high quality settings, written as 10-bit 4:4:4 HEVC; otherwise output is 8-bit H.264. Those settings use software decoding, because hardware decoders may drop to 8 bits; the fast setting uses hardware decoding and encoding.

Each frame is scrambled whole and then matched and range-handled against itself. Four temporal modes:

- **Fixed pattern** (default): every frame uses the base seed, so the same phase field, warp or permutation is applied throughout. Moving content passes through a still scramble.
- **New each frame:** frame *i* gets its own derived seed, giving dynamic noise.
- **Spatiotemporal:** frames are phase scrambled together. Each channel's block of frames is a volume (*x*, *y*, *t*) whose 3D Fourier transform gets one random phase field (Hermitian, zero at DC, scaled by the intensity, shared by all channels). This keeps the 3D amplitude spectrum, so both spatial and temporal frequency content, and each channel's mean, while destroying spatial and temporal structure. The volume is treated as periodic in space and time. Videos that do not fit the memory budget (4 GB by default) are split into equal volumes with derived seeds; consecutive volumes overlap by 8 frames and are crossfaded with equal-power weights (deviations from the frame means mixed by cos(π*u*/2) and sin(π*u*/2), means mixed linearly) so contrast stays constant. The number of volumes is reported. The scramble method is not used in this mode.
- **Follow motion:** the scramble moves with the content as an advected texture (Max & Becker, 1996; Neyret, 2003), driven by SEA-RAFT optical flow. I use the bidirectional scheme of Bousseau et al. (2007), with a few additions of my own:
  - **Layers.** The video is processed in intervals of *N* frames (the refresh interval). A forward layer starts from the scramble of the interval's first frame and is carried forward; a reverse layer starts from the first frame of the next interval and is carried backward. Their weights at frame *t* are cos²(π*t*/2*N*) and sin²(π*t*/2*N*). Background uncovered by motion has no past but has a future, so the reverse layer covers it. Each layer stores a position per pixel and samples its scramble once (Catmull-Rom), so the texture never blurs. Short intervals stretch less; long ones read better as motion.
  - **Flow.** Estimated in both directions between consecutive frames, smoothed with a Gaussian (σ = 1% of the shorter side).
  - **Trust.** A layer is weighted down where it is distorted: with *s₁ ≥ s₂* the singular values of its Jacobian, *d* = max(*s₁*, 1/*s₂*) and the weight is multiplied by 1/(1 + ((*d* − 1)/0.25)²). It is also dropped, for the rest of the interval, wherever a pixel's path fails a round trip (flow forward then back misses by more than 1 px + 5% of the motion; weight 0 at twice that), which marks content covered, uncovered or leaving the frame. The current frame's own scramble enters with weight 0.25 and takes over where both layers fail.
  - **Blending.** Layers mix without losing contrast: deviations from the mean are divided by √(*a*² + *b*² + 2*abρ*), with *ρ* their local correlation (Gaussian window, 3% of the shorter side).
  - **Kept part.** What a partial scramble leaves unchanged (per channel, the least-squares gain Σ Re(*S*·*O*\*) / Σ |*O*|² over 48 radial bands, limited to 0 to 1) always comes from the current frame; only the changed part is carried. A Fourier frequency range or a blur never shows other frames.
  - **Pixel and block.** Whole cells (pixels or blocks) move on the frame's grid and mix by a dissolve, each cell taken from one source by a fixed random threshold. Moving areas stay a mosaic of intact cells, moving in steps of one cell, so a paused frame shows no seam.
  - **Diffeomorphic.** I carry the warp's displacement field instead of the warped image, blend fields keeping their local RMS, and smooth the result (σ = 1%) so the warp bends rather than tears between regions moving differently. Each output is a warp of its own frame.
  - **Local phase.** I carry each band's random phase offsets, blended as unit phasors, so each output keeps the local amplitudes of its own frame.

  Moving objects still show as moving regions: this mode keeps motion by design and is not meant for anonymisation. With smooth textures (Fourier, local phase), a paused frame can show the edge of a moving region, because large-scale structure cannot match across it; pixel scrambling hides it best.

**Faces in video.** Faces are detected in every frame and the scrambled frame shows only inside the face masks; outside, the original (Keep) or black (Remove). Scrambling the whole frame rather than each face keeps the pattern stable as faces move and resize. A frame without a detected face reuses the previous mask for up to 15 frames (moved along the flow in follow motion); the count of such frames is reported. In follow motion the layers only sample scrambles, never the output, so unscrambled content cannot be carried into a face.

## Face detection

Faces are found with the UltraFace detector (640 × 480 input, default confidence 0.7). Each box, scaled by the region size, is scrambled with its own seed and blended in with a weight *w*: output = *w* · scrambled + (1 − *w*) · original.

**Mask shape.** With *u*, *v* from −1 to 1 across the box, the distance from the centre is *d* = max(|*u*|, |*v*|) for a rectangle or √(*u*² + *v*²) for the inscribed ellipse. With *f* the soft edge fraction:

$$w = \begin{cases} 1 & d \le 1 - f \\ t^2 (3 - 2t), \quad t = (1 - d)/f & 1 - f < d < 1 \\ 0 & d \ge 1 \end{cases}$$

For the **outline** shape, a square crop around the box (1.5 times its larger side) goes to the MediaPipe Face Mesh V2 landmark model (256 × 256 input); its 36 face-oval landmarks form a polygon, scaled about its centroid by the region size. Inside it, *t* is the distance to the nearest edge divided by *f* times half the smaller side of the polygon's bounding box.

- Both models run on a single CPU thread, so masks are identical on every machine.
- A soft edge hides the region's outline but leaves more of the original visible towards the edge.
- *Keep* leaves the rest of the image unchanged; *Remove* outputs only the scrambled faces on a transparent background, with *w* as alpha.
- The number of faces is reported; with none, the image is returned unchanged.

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
| SSIM | Mean structural similarity of the two luminance images (Wang et al., 2004), 11 × 11 Gaussian window with σ = 1.5, $C_1 = 0.01^2$, $C_2 = 0.03^2$. |

## Normalization

Normalization treats all images in a folder as one set, following the SHINE toolbox (Willenbockel et al., 2010). Values outside 0 to 1 are clipped and reported per image.

- **Luminance.** Each image is mapped by $v' = (v - \mu)\,\sigma_t/\sigma + \mu_t$ on every channel, with μ and σ its luminance mean and standard deviation and $\mu_t$, $\sigma_t$ the target (by default the set averages). The luma weights sum to one, so every image's luminance then has exactly that mean and standard deviation (before clipping).
- **Histogram.** Per channel, the target is the set average of the sorted pixel values (quantile averaging). Each image's pixels are ranked, ties broken by position, and given the target value at the same quantile: exact histogram specification.
- **Spectrum.** Per channel, each image's amplitude spectrum is replaced by the set average while its phase is kept. Images must have the same size.

## References

- Bousseau, A., Neyret, F., Thollot, J., & Salesin, D. (2007). Video watercolorization using bidirectional texture advection. *ACM Transactions on Graphics*, 26(3), 104. https://doi.org/10.1145/1275808.1276507
- Max, N., & Becker, B. (1996). Flow visualization using moving textures. *Proceedings of the ICAS/LaRC Symposium on Visualizing Time-Varying Data*, 77–87.
- Neyret, F. (2003). Advected textures. *Proceedings of the ACM SIGGRAPH/Eurographics Symposium on Computer Animation*, 147–153. https://doi.org/10.2312/SCA03/147-153
- Portilla, J., & Simoncelli, E. P. (2000). A parametric texture model based on joint statistics of complex wavelet coefficients. *International Journal of Computer Vision*, 40(1), 49–70.
- Stojanoski, B., & Cusack, R. (2014). Time to wave good-bye to phase scrambling: Creating controlled scrambled images using diffeomorphic transformations. *Journal of Vision*, 14(12):6.
- Wang, Z., Bovik, A. C., Sheikh, H. R., & Simoncelli, E. P. (2004). Image quality assessment: From error visibility to structural similarity. *IEEE Transactions on Image Processing*, 13(4), 600–612.
- Willenbockel, V., Sadr, J., Fiset, D., Horne, G. O., Gosselin, F., & Tanaka, J. W. (2010). Controlling low-level image properties: The SHINE toolbox. *Behavior Research Methods*, 42(3), 671–684.
