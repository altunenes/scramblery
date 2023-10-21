
# Scramblery Functions in Python

Maintained by: @altunenes

This document outlines the updated functionalities of the Scramblery library, including the newly introduced Fourier scrambling technique and the enhancements made to the video scrambling function.

## Updates

1. **Fourier Scrambling**: A novel method for image scrambling that operates in the frequency domain. Unlike traditional scrambling methods that shuffle pixel values or coordinates, Fourier scrambling randomizes the phase information of the image's Fourier transform, thereby disrupting the image's structure while preserving its energy distribution. This method is particularly potent for applications requiring non-reversible image transformations without a significant loss of information content (e.g., privacy preservation, data anonymization).

2. **Video Scrambling Enhancements**: The `scramblevideo` function has been overhauled to provide more flexibility and control over the scrambling process. Users can now specify detailed scrambling settings, allowing for different scrambling techniques to be applied throughout a video sequence. This feature is crucial for researchers and practitioners working with video data, where consistent scrambling parameters are necessary across frames.

## Function Documentation

### `scrambleface`

```python
def scrambleface(img, splits, type, seamless=False, bg=True, seed=None, write=True, scramble_ratio=1.0):
    """
Scramble the facial area of the image.

Args:
    img: input image path or image array
    splits: number of splits to perform, works differently between square and stack splits
    type: type of split, "pixel", "stack", or "fourier". Stack works with pixel coordinates, square works with pixel values
    seamless: if True, uses seamlessClone to blend the scrambled face with the original image
    bg: if True, uses the background of the original image, if False, replaces the background with a gray color
    seed: seed for random number generator (default: None)
    write: if True, writes the output image to disk, if False, returns the output image
    scramble_ratio: the power of Fourier scrambling (default: 1.0)

Usage:
    scrambleface("image.png", 10, "fourier", False, True, None, True, 0.5)
    """
```

### `scrambleimage`

```python
def scrambleimage(image_path, x_block=10, y_block=10, scramble_type='classic', seed=None, write=True, **kwargs):
    """
Main function to scramble an image based on the specified scramble type.

Args:
    image_path: Path to the input image (with extension).
    x_block: Number of splits in x-axis.
    y_block: Number of splits in y-axis.
    scramble_type: Type of split; valid values are "classic", "pixel", "withinblocks", "rotate", "colormap", "gradient", "fourier".
    seed: Seed for random number generator (default: None).
    write: Write the image to disk (default: True).
    **kwargs: Additional keyword arguments specific to scramble types.

Returns:
    If write is True, writes the scrambled image to disk.
    If write is False, returns the scrambled image.

Raises:
    ValueError: If an invalid scramble_type is provided or image cannot be read.
    FileNotFoundError: If the image_path does not exist.
    """
```

### `scramblevideo`

```python
def scramblevideo(input_video_path, output_video_path=None, scramble_settings=None):
    """
Scramble the facial area in a video.

Args:
    input_video_path: Path to the input video.
    splits: Number of splits to perform on the face.
    output_video_path: Path where the output video will be saved (if None, the video won't be saved).
    scramble_settings: Dictionary of settings for the scrambleface function.

Usage:
    scramblevideo("input_video.mp4", 10, "output_video.mp4", scramble_settings)
    """
```

## Usage Examples

### Fourier Scrambling

The `scrambleface` and `scrambleimage` functions now support Fourier scrambling, a method that disrupts the image's visual content without altering its energy spectrum significantly. This technique is particularly useful in situations where the image's overall energy distribution must be maintained for subsequent processing steps (e.g., deep learning applications) or for privacy-preserving transformations.

```python
from scramblery import scrambleface, scrambleimage

# Scramble the face in the image using the Fourier method
scrambled_face = scrambleface("face.jpg", splits=10, type="fourier", scramble_ratio=0.7, write=False)

# Scramble the entire image using the Fourier method
scrambled_image = scrambleimage("image.jpg", scramble_type="fourier", scramble_ratio=0.5, write=False)
```

### Enhanced Video Scrambling

The updated `scramblevideo` function allows for more granular control over the video scrambling process, with the ability to specify detailed settings for the `scrambleface` function used internally. This update is particularly beneficial for use cases requiring consistent scrambling effects across different video frames or varied scrambling intensities throughout a video sequence.

```python
from scramblery import scramblevideo

# Define custom settings for the video scrambling process
scramble_settings = {
    'splits': 20,
    'type': 'fourier',
    'scramble_ratio': 0.6,
    'seamless': True,
    'bg': False
}

# Apply scrambling to a video file
scramblevideo("input_video.mp4", "output_scrambled_video.mp4", scramble_settings)
```

---

