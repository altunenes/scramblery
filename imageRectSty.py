     
 """
# ==========================================================================================================================================================
#                                                                       Just Run The Script. Scramble method == Square
# ==========================================================================================================================================================
"""
     


import cv2
import numpy as np

#Create an Image Scrambler script
def scramble(image, parts):
    #Shuffle the parts of the image's randomly
    h, w, c = image.shape
    w1 = w//parts
    h1 = h//parts
    idx = list(range(parts*parts))
    np.random.shuffle(idx)
    for i in range(parts):
        for j in range(parts):
            x = idx[i*parts+j]%parts
            y = idx[i*parts+j]//parts
            roi = image[y*h1:(y+1)*h1, x*w1:(x+1)*w1]
            image[y*h1:(y+1)*h1, x*w1:(x+1)*w1] = image[i*h1:(i+1)*h1, j*w1:(j+1)*w1]
            image[i*h1:(i+1)*h1, j*w1:(j+1)*w1] = roi
    return image

#Get the desired numbers of shuffle parts from the user
parts = int(input("Enter the number of parts: "))

#Load the image
img = cv2.imread("Lena.png")

#Call the function
scrambled = scramble(img, parts)

#Show the scrambled image
cv2.imshow("Scrambled", scrambled)
cv2.waitKey(0)
cv2.destroyAllWindows()
