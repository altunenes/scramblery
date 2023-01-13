###API reference
This page gives an overview of the functions 
+[get_facial_landmarks](#get_facial_landmarks)

+[scrambleface](#scrambleface)

+[scrambleimage](#scrambleimage)

+[scramblevideo](#scramblevideo)

***get_facial_landmarks***<a name="get_facial_landmarks"></a>
the get_facial_landmarks function takes an image and returns the facial landmarks of the face in the image.

***scrambleface***<a name="scrambleface"></a>
the scrambleface function takes an image and returns a scramble the facial area of the image. You can either scramble with the pixel values or with the pixel coordinate

```
    """"scramble_face: Scramble the facial area of image.
    Args:
        img: input image
        splits: number of splits to perform, it works reversed between pixel and stack splits
        type: type of split, "pixel" or "stack". Stack works with pixel coordinates, square works with pixel values
        seamless: if True, it will use seamlessClone to blend the scrambled face with the original image
        bg: if True, it will use the background of the original image, if False, it will replace the background with a gray color
        seed: seed for random number generator (default: None)
        write: if True, it will write the output image to the disk, if False, it will return the output image
    """
```

usage:
```python
    scrambleface("test.jpg",splits=12,type="pixel",bg=True,seamless=True,write=True)
```
In this version, with write=True, the function will write the scrambled image to the disk. If you want to use the scrambled image in your code, you can set write=False and the function will return the scrambled image.

```python
    import cv2
    img=cv2.imread("test.jpg")
    a=scrambleface(img,splits=12,type="pixel",bg=True,seamless=True,write=False)
    cv2.imshow("scrambled",a)
    cv2.waitKey(0)
```

The function will return the scrambled image. You can use the image in your code. This is useful if you want to scramble multiple images in a loop. Check the user guide for more information, I show how to use the function in a loop.

Note: bg=False and seamless=True will not work. seamlessClone requires a background image. If you want to use seamlessClone, you need to set bg=True. I may add this feature in the future.


***scrambleimage***<a name="scrambleimage"></a>
the scrambleimage function takes an image and returns a scrambled version of the image. This function is for the whole image.
types:"pixel","square","withinblocks","rotate","colormap","gradient"

note: gradient is a implement a scramble type that randomly applies a gradient to blocks of the image. This could be achieved using the cv2.Sobel or cv2.Laplacian functions. colormap, will randomly apply one of the available color maps (Autumn, Bone, Jet, Winter, Rainbow, or Ocean) to each block of the image when the function is called with the "colormap" scramble type. This two parameters don't shuffle the blocks.

```
    """scramble_image: Scramble the whole image.
    Args:
        image: input image(with extension)
        x_block: number of splits in x-axis
        y_block: number of splits in y-axis
        type: type of split: "pixel","classic","withinblocks"
        seed: seed for random number generator
        write: write the image to disk or return the scrambled image(True is default)
        Usage:
            scrambleimage("image.png",10,10,"pixel")
    """
```


***scramblevideo***<a name="scramblevideo"></a>
the scramblevideo function takes a video and returns a scrambled version of the video.