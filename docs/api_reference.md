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

```
    """scramble_image: Scramble the whole image.
    Args:
        image: input image(with extension)
        x_block: number of splits in x-axis
        y_block: number of splits in y-axis
        type: type of split, "pixel" or "classic" square is recommended to maintain the image dimensions. classic is works with pixel coordinates, pixel works with pixel values
        Usage:
            scrambleimage("image.png",10,10,"pixel")
    """
```

***scramblevideo***<a name="scramblevideo"></a>
the scramblevideo function takes a video and returns a scrambled version of the video.