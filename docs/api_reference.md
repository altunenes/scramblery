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
        Usage:
            scrambleimage("image.png",10,10,"pixel")
    """
```

***scramblevideo***<a name="scramblevideo"></a>
the scramblevideo function takes a video and returns a scrambled version of the video.