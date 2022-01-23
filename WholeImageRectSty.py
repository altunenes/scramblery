     
 """
# ==========================================================================================================================================================
#                                                                       Whole Image Scrambler
                                                                         Author: Enes Altun
                                          Usage:go to the 53'th line and enter the name of the image and desired num. of squares.
# ==========================================================================================================================================================
"""


import cv2
import numpy as np

def scramble_image_data(image, x_block=10, y_block=10):
    #Create a list of x_block number of lists
    x_length = image.shape[1]
    y_length = image.shape[0]
    x_list = []
    for i in range(x_block):
        x_list.append(image[0:y_length, int(x_length/x_block) * i:int(x_length/x_block) * (i+1)])
    #Shuffle the lists
    np.random.shuffle(x_list)
    #Join the lists to each other
    new_image = np.concatenate(x_list, axis=1)
    #Create a list of y_block number of lists
    x_list = []
    for i in range(y_block):
        x_list.append(new_image[int(y_length/y_block) * i:int(y_length/y_block) * (i+1), 0:x_length])
    #Shuffle the lists
    np.random.shuffle(x_list)
    #Join the lists to each other
    new_image = np.concatenate(x_list, axis=0)
    #Return the new image
    return new_image

#Apply the full image to the new picture
def scramble_image(image_path, x_block=10, y_block=10):
    #Load the image
    image = cv2.imread(image_path)
    #Scramble the image
    new_image = scramble_image_data(image, x_block, y_block)
    #get the image name
    image_name = image_path.split('/')[-1]
    #get the image extension
    image_name = image_name.split('.')[0]
    print(image_name)
    #Save the image
    cv2.imwrite(f'{image_name}SCRAMBLED_'+f'{x_block,y_block}.png', new_image)

#24 square on the X-axis, 24 square on the Y-axis

#enter your image name
scramble_image("Lena.png", 24, 24) 
