#############################      SCRAMBLERY FUNCTIONS IN PYTHON   #####################################
#############################      MAINTAINED BY: @altunenes        #####################################

import mediapipe as mp
import cv2
import numpy as np
import os
mp_face_mesh = mp.solutions.face_mesh
face_mesh = mp_face_mesh.FaceMesh()

def get_facial_landmarks(frame):
    """a function for detecting facial landmarks"""
    height, width, _ = frame.shape
    frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
    result = face_mesh.process(frame_rgb)

    facelandmarks = []
    for facial_landmarks in result.multi_face_landmarks:
        for i in range(0, 468):
            pt1 = facial_landmarks.landmark[i]
            x = int(pt1.x * width)
            y = int(pt1.y * height)
            facelandmarks.append([x, y])
    return np.array(facelandmarks, np.int32)
def scrambleface(img,splits,type,seamless=False,bg=True):
    """"scramble_face: Scramble the facial area of image.
    Args:
        img: input image
        splits: number of splits to perform, it works reversed between square and stack splits
        type: type of split, "square" or "stack". Stack works with pixel coordinates, square works with pixel values
        seamless: if True, it will use seamlessClone to blend the scrambled face with the original image
        bg: if True, it will use the background of the original image, if False, it will replace the background with a gray color

    usage:
        scrambleface("image.png",10,"square",False,True)
    """
    img_name = os.path.splitext(os.path.basename(img))[0]
    img = cv2.imread(img)
    img_copy = img.copy()
    h, w, _ = img.shape
    mask = np.zeros((h, w), np.uint8)
    landmarks = get_facial_landmarks(img)
    hull = cv2.convexHull(landmarks)
    cv2.fillConvexPoly(mask, hull, 255)

    if type=="stack":

        for i in range(0, 468):
            pt = landmarks[i]

        splits=int(splits)
        img_new = np.array_split(img, splits)
        np.random.shuffle(img_new)

        img_new = np.vstack(img_new)

        face = cv2.bitwise_and(img_new, img_new, mask=mask)
        frame_copy = img_new
        face_extracted = cv2.bitwise_and(frame_copy, frame_copy, mask=mask)

        backgroundm = cv2.bitwise_not(mask)
        background = cv2.bitwise_and(img, img, mask=backgroundm)

        result = cv2.add(background, face_extracted)
        cv2.imwrite(f'{img_name}_scrambled_stack.png', result)

    elif type=="square":
        for i in range(0,splits):
            for j in range(0,splits):
                x1=int((i/splits)*w)
                y1=int((j/splits)*h)
                x2=int(((i+1)/splits)*w)
                y2=int(((j+1)/splits)*h)

                img[y1:y2,x1:x2]=img[np.random.randint(y1,y2),np.random.randint(x1,x2)]
                #img[y1:y2,x1:x2]=cv2.bitwise_and(img[y1:y2,x1:x2],img[y1:y2,x1:x2],mask=mask[y1:y2,x1:x2]) #this also works, but to adjust the b

        out = 128*np.ones_like(img)
        out[mask == 255] = img[mask == 255]
        if bg == False:
            cv2.imwrite(f'{img_name}_scrambled_nobg.png', out)
        if seamless==True:
            out = cv2.seamlessClone(out, img_copy, mask, (w//2, h//2), cv2.NORMAL_CLONE)
            cv2.imwrite(f'{img_name}scrambled_seamless.png', out)

        else:
            img_copy[mask == 255] = out[mask == 255]
            cv2.imwrite(f'{img_name}_scrambled_square.png', img_copy)
            
def scramble_image_data(image, x_block=10, y_block=10):
    """a function for further use"""
    x_length = image.shape[1]
    y_length = image.shape[0]
    x_list = []
    for i in range(x_block):
        x_list.append(image[0:y_length, int(x_length/x_block) * i:int(x_length/x_block) * (i+1)])
    np.random.shuffle(x_list)
    new_image = np.concatenate(x_list, axis=1)
    x_list = []
    for i in range(y_block):
        x_list.append(new_image[int(y_length/y_block) * i:int(y_length/y_block) * (i+1), 0:x_length])
    np.random.shuffle(x_list)
    new_image = np.concatenate(x_list, axis=0)
    return new_image


def scramble_image_data2(image_path, x_block=10, y_block=10):
    """a function for further use"""
    image_name = image_path.split('/')[-1]
    image_name = image_name.split('.')[0]
    image = cv2.imread(image_path)
    new_image = scramble_image_data(image, x_block, y_block)

    print("your image has been scrambled with dimensions",x_block,"x",y_block)
    cv2.imwrite(f'{image_name}SCRAMBLED_' + f'{x_block, y_block}.png', new_image)



def scrambleimage(image,x_block,y_block,type):
    """scramble_image: Scramble the whole image.
    Args:
        image: input image(with extension)
        x_block: number of splits in x-axis
        y_block: number of splits in y-axis
        type: type of split, "square" or "classic" square is recommended to maintain the image dimensions. classic is works with pixel coordinates, square works with pixel values
        Usage:
            scrambleimage("image.png",10,10,"square")
    """
    if type == "classic":
        x_block=int(x_block)
        y_block=int(y_block)
        scramble_image_data2(image,x_block,y_block)
    elif type == "square":
        img_name = os.path.splitext(os.path.basename(image))[0]
        image = cv2.imread(image)

        h, w, _ = image.shape
        for i in range(0,x_block):
            for j in range(0,y_block):
                x1=int((i/x_block)*w)
                y1=int((j/y_block)*h)
                x2=int(((i+1)/x_block)*w)
                y2=int(((j+1)/y_block)*h)

                image[y1:y2,x1:x2]=image[np.random.randint(y1,y2),np.random.randint(x1,x2)]
        
        cv2.imwrite(f'{img_name}_Wscrambled_square.png', image)

def scramblevideo(cap,splits):
    """scramble_video: Scramble the whole video.
    Args:
        cap: input video
        splits: number of splits to perform
    Usage:
        scramblevideo("video.mp4",10)

        """
    cap = cv2.VideoCapture(cap)
    splits=int(splits)
    while True:

        ret, frame = cap.read()
        frame_copy = frame.copy()
        h, w, _ = frame.shape
        mask = np.zeros((h, w), np.uint8)
        landmarks = get_facial_landmarks(frame)
        print(landmarks)
        hull = cv2.convexHull(landmarks)
        cv2.fillConvexPoly(mask, hull, 255)

        cv2.polylines(frame, [hull], 1, (0, 0, 255), 1)

        cv2.polylines(mask, [hull], 1, (255), 1)

        for i in range(0, 468):
            pt = landmarks[i]
        if ret is not True:
            break

        # frame_copy = cv2.blur(frame_copy, (27, 27)) # testing blur
        img_new = np.array_split(frame_copy, splits)
        np.random.shuffle(img_new)
        img_new = np.vstack(img_new)
        #face = cv2.bitwise_and(img_new, img_new, mask=mask) further use
        frame_copy = img_new
        face_extracted = cv2.bitwise_and(frame_copy, frame_copy, mask=mask)

        backgroundm = cv2.bitwise_not(mask)
        background = cv2.bitwise_and(frame, frame, mask=backgroundm)

        result = cv2.add(background, face_extracted)

        cv2.imshow("mask", result)

        cv2.waitKey(1)


