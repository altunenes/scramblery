#### Installation
- The package can be found in pypi. To install the package, run the following command in the terminal:
- `pip install scramblery`

### Dependencies
To use the `scramblery` package, the following Python packages need to be installed. You can install them using `pip`:

```sh
pip install mediapipe
pip install opencv-python
pip install numpy
```

#### Usage
- import the package:
- `from scramblery import scramblery`
- The package has 3 functions to use. Use the functions in the following manner:
- `scramblery.scrambleimage("Lena.png",8,8)`
- `scramblery.scrambleface("Lena.png",8)`
  
Sometimes there may be some errors related to the size of the image when splitting into squares. Therefore, you may need to edit the number you will reserve. I will fix this issue in future releases.



If you want to scramble multiple images you can use following code for best results. This will make sure that the same blocks are scrambled in all images.

```python
import glob
#input folder containing the images to be scrambled (in this case, the folder is called 'inputfolder' in my working directory)
input_dir = 'inputfolder'
#output folder where the scrambled images will be saved 
output_dir = 'output'
image_files = glob.glob(os.path.join(input_dir, '*.*'))
for image_file in image_files:
    #scramble the image but don't write it to disk (write=False)
    image=scrambleimage(image_file, scramble_type='pixel', write=False)
    #get the base name of the image file
    base_name = os.path.splitext(os.path.basename(image_file))[0]
    #create the output file name by appending '_scrambled.jpg' to the base name. You can change the extension to .png or .jpg
    output_file = os.path.join(output_dir, base_name + '_scrambled.png')
    #write the scrambled image to disk (using the output file name)
    cv2.imwrite(output_file, image)
```

Note that, this is only for the new output folder. If you want to scramble the images in your working directory it is more simple. You can use the following code.

```python
import glob
input_dir = 'inputfolder'
image_files = glob.glob(os.path.join(input_dir, '*.*'))
for image_file in image_files:
    #scramble the images and write them into the your working directory.
    image=scrambleimage(image_file, scramble_type='pixel', write=True)
```

This was a simple example of how to use the scrambleimage function to scramble multiple images. You can easily modify the code to suit your needs.


### Scrambling Faces in Videos


The `scramblevideo` function allows you to scramble faces within a video. It's perfect for studies involving dynamic stimuli or any application where you need to anonymize faces in motion.

Here's how to use it: 

```python

from scramblery import scramblery

# Path to the input video
input_video_path = "enes23.mp4"

# Path where the output video will be saved
output_video_path = "enes2555.mp4"

# Scramble settings
scramble_settings = {
    'splits': 25,         # Number of splits to perform on the face
    'type': 'pixel',      # Type of scrambling ('pixel', 'classic', etc.)
    'seamless': False,    # Whether to use seamless cloning (True or False)
    'bg': True,           # Whether to include background scrambling (True or False)
    'seed': None,         # Seed for random number generator (None for random seed)
    'write': False        # Must always be False for video processing
}

# Call the function
scramblery.scramblevideo(input_video_path, output_video_path, scramble_settings)
```

### Detailed Explanation

- **input_video_path**: The path to the video file you wish to process.
- **output_video_path**: The path where the scrambled video will be saved. If you don't want to save the video, set this to `None`.
- **scramble_settings**: A dictionary of settings that dictate how the video will be scrambled.
  - **splits**: Determines how many blocks the face will be split into. A higher number means more, smaller blocks.
  - **type**: The type of scrambling. Options include 'pixel', 'classic', 'fourier', etc.
  - **bg**: If `True`, the background will also be scrambled, not just the face.
  - **seed**: A seed for the random number generator, ensuring the same scrambling is applied across different calls. If `None`, the seed will be random.
  - **write**: This should always be `False` for video processing, as the function handles the writing process.

### Performance Note
Processing videos, especially long ones, can be time-consuming and computationally expensive. The time it takes to process a video largely depends on the length of the video, the resolution, the `splits` parameter, and the hardware of your computer. Ensure your system is adequately equipped to handle the processing, and expect this operation to be more time-consuming than processing images.
