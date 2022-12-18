#### Installation
- The package can be found in pypi. To install the package, run the following command in the terminal:
- `pip install scramblery`
#### Usage
- import the package:
- `from scramblery import scramblery`
- The package has 3 functions to use. Use the functions in the following manner:
- `scramblery.scrambleimage("Lena.png",8,8)`
- `scramblery.scrambleface("Lena.png",8)`
-  scramblery.scramblevideo("Lena.png",8,8)
- 
Sometimes there may be some errors related to the size of the image when splitting into squares. Therefore, you may need to edit the number you will reserve. I will fix this issue in future releases.



If you want to scramble multiple images you can use following code for best results. This will make sure that the same blocks are scrambled in all images.

```
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