#############################      SCRAMBLERY FUNCTIONS IN PYTHON   #####################################
#############################      MAINTAINED BY: @altunenes        #####################################

import mediapipe as mp
import cv2
import numpy as np
import os
mp_face_mesh = mp.solutions.face_mesh
face_mesh = mp_face_mesh.FaceMesh()
import random

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
def scrambleface(img,splits,type,seamless=False,bg=True,seed=None,write=True):
    """"scramble_face: Scramble the facial area of image.
    Args:
        img: input image
        splits: number of splits to perform, it works reversed between square and stack splits
        type: type of split, "pixel" or "stack". Stack works with pixel coordinates, square works with pixel values
        seamless: if True, it will use seamlessClone to blend the scrambled face with the original image
        bg: if True, it will use the background of the original image, if False, it will replace the background with a gray color
        seed: seed for random number generator (default: None)
        write: if True, it will write the output image to the disk, if False, it will return the output image

    usage:
        scrambleface("image.png",10,"square",False,True)
    """
    if seed is not None:
        np.random.seed(seed)

    if type not in ["stack","pixel"]:
        raise ValueError("type must be stack or pixel")
    if bg==False and seamless==True:
        print("You can't have seamless without bg")
        seamless=False
        

    if write==True:
        image_path = img.split('/')[-1]
        image_path = img.split('.')[0]
        img_name = os.path.splitext(os.path.basename(image_path))[0]

        img = cv2.imread(img)    
    img_copy = img.copy()
    h, w, _ = img.shape
    mask = np.zeros((h, w), np.uint8)
    landmarks = get_facial_landmarks(img)
    hull = cv2.convexHull(landmarks)
    cv2.fillConvexPoly(mask, hull, 255)

    if type=="stack":
        splits=int(splits)
        img_new = np.array_split(img, splits)
        np.random.shuffle(img_new)
        img_new = np.vstack(img_new)
        face = cv2.bitwise_and(img_new, img_new, mask=mask)
        face_only = 128*np.ones_like(img)
        face_only[mask == 255] = face[mask == 255]
        frame_copy = img_new
        face_extracted = cv2.bitwise_and(frame_copy, frame_copy, mask=mask)

        backgroundm = cv2.bitwise_not(mask)
        background = cv2.bitwise_and(img, img, mask=backgroundm)

        result = cv2.add(background, face_extracted)
        if seamless==True:
            M = cv2.moments(hull)
            cX = int(M["m10"] / M["m00"])
            cY = int(M["m01"] / M["m00"])
            center = (cX, cY)
            result = cv2.seamlessClone(result, img_copy, mask, center, cv2.NORMAL_CLONE)
            if write:
                cv2.imwrite(f'{img_name}SCRAMBLED_seamless' + f'{splits}.png', result)
            else:
                return result
        if bg == False:
            if write:
                cv2.imwrite(f'{img_name}SCRAMBLED_nobg' + f'{splits}.png', face_only)
            else:
                return face_only
        elif seamless==False:
            img_copy[mask == 255] = result[mask == 255]
            if write:
                cv2.imwrite(f'{img_name}SCRAMBLED_' + f'{splits}.png', img_copy)
            else:
                return img_copy

    elif type=="pixel":
        for i in range(0,splits):
            for j in range(0,splits):
                x1=int((i/splits)*w)
                y1=int((j/splits)*h)
                x2=int(((i+1)/splits)*w)
                y2=int(((j+1)/splits)*h)

                img[y1:y2,x1:x2]=img[np.random.randint(y1,y2),np.random.randint(x1,x2)]

        out = 128*np.ones_like(img)
        out[mask == 255] = img[mask == 255]
        if seamless==True:
            M = cv2.moments(hull)
            cX = int(M["m10"] / M["m00"])
            cY = int(M["m01"] / M["m00"])
            center = (cX, cY)
            out = cv2.seamlessClone(out, img_copy, mask, center, cv2.NORMAL_CLONE)
            if write:
                cv2.imwrite(f'{img_name}SCRAMBLED_seamless' + f'{splits, splits}.png', out)
            else:
                return out
        if bg == False:
            if write:
                cv2.imwrite(f'{img_name}SCRAMBLED_nobg' + f'{splits, splits}.png', out)
            else:
                return out
        elif seamless==False:
            img_copy[mask == 255] = out[mask == 255]
            if write:
                cv2.imwrite(f'{img_name}SCRAMBLED_' + f'{splits, splits}.png', img_copy)

            else:
                return img_copy


def scrambleimage(image, x_block=10, y_block=10, scramble_type='classic',seed=None,write=True):
    """scramble_image: Scramble the whole image.
    Args:
        image: input image(with extension)
        x_block: number of splits in x-axis
        y_block: number of splits in y-axis
        type: type of split, "pixel","square","withinblocks","rotate","colormap","gradient"
        seed: seed for random number generator (default: None)
        write: write the image to disk (default: True) 
        Usage:
            scrambleimage("image.png",10,10,"pixel")
    """
    if seed is not None:
        np.random.seed(seed)
    
    if write==True:
        image_path = image.split('/')[-1]
        image_path = image.split('.')[0]
        image = cv2.imread(image)
    def scramble_image_data(image, x_block, y_block):
        h, w, _ = image.shape
        block_width = w // x_block
        block_height = h // y_block
        blocks = []
        for i in range(y_block):
            for j in range(x_block):
                y1 = i * block_height
                y2 = (i+1) * block_height
                x1 = j * block_width
                x2 = (j+1) * block_width
                blocks.append(image[y1:y2, x1:x2])
        np.random.shuffle(blocks)
        new_image = np.zeros((h, w, 3), dtype=np.uint8)
        k = 0
        for i in range(y_block):
            for j in range(x_block):
                y1 = i * block_height
                y2 = (i+1) * block_height
                x1 = j * block_width
                x2 = (j+1) * block_width
                new_image[y1:y2, x1:x2] = blocks[k]
                k += 1
        return new_image
    
    if scramble_type == 'classic':
        x_block = int(x_block)
        y_block = int(y_block)
        new_image = scramble_image_data(image, x_block, y_block)
    elif scramble_type == 'pixel':
        h, w, _ = image.shape
        for i in range(0, x_block):
            for j in range(0, y_block):
                x1 = int((i/x_block)*w)
                y1 = int((j/y_block)*h)
                x2 = int(((i+1)/x_block)*w)
                y2 = int(((j+1)/y_block)*h)
                image[y1:y2, x1:x2] = image[np.random.randint(y1, y2), np.random.randint(x1, x2)]
        new_image = image
    elif scramble_type == 'withinblocks':
        h, w, _ = image.shape
        block_width = w // x_block
        block_height = h // y_block
        new_image = np.zeros((h, w, 3), dtype=np.uint8)
        for i in range(y_block):
            for j in range(x_block):
                y1 = i * block_height
                y2 = (i+1) * block_height
                x1 = j * block_width
                x2 = (j+1) * block_width
                block = image[y1:y2, x1:x2]
                block = block[np.random.permutation(block_height), :]
                block = block[:, np.random.permutation(block_width)]
                new_image[y1:y2, x1:x2] = block

    elif scramble_type == 'rotate':
        h, w, _ = image.shape
        block_width = w // x_block
        block_height = h // y_block
        new_image = np.zeros((h, w, 3), dtype=np.uint8)
        for i in range(y_block):
            for j in range(x_block):
                y1 = i * block_height
                y2 = (i+1) * block_height
                x1 = j * block_width
                x2 = (j+1) * block_width
                block = image[y1:y2, x1:x2]
                # Rotate the block by a random number of degrees between -45 and 45
                angle = random.uniform(-45, 45)
                rows, cols = block.shape[:2]
                M = cv2.getRotationMatrix2D((cols/2, rows/2), angle, 1)
                block = cv2.warpAffine(block, M, (cols, rows))
                new_image[y1:y2, x1:x2] = block    
    elif scramble_type == 'colormap':
        h, w, _ = image.shape
        block_width = w // x_block
        block_height = h // y_block
        new_image = np.zeros((h, w, 3), dtype=np.uint8)
        for i in range(y_block):
            for j in range(x_block):
                y1 = i * block_height
                y2 = (i+1) * block_height
                x1 = j * block_width
                x2 = (j+1) * block_width
                block = image[y1:y2, x1:x2]
                available_maps = [cv2.COLORMAP_AUTUMN, cv2.COLORMAP_BONE, cv2.COLORMAP_JET,
                                 cv2.COLORMAP_WINTER, cv2.COLORMAP_RAINBOW, cv2.COLORMAP_OCEAN]
                chosen_map = random.choice(available_maps)
                block = cv2.applyColorMap(block, chosen_map)
                new_image[y1:y2, x1:x2] = block
    elif scramble_type == 'gradient':
        h, w, _ = image.shape
        block_width = w // x_block
        block_height = h // y_block
        new_image = np.zeros((h, w, 3), dtype=np.uint8)
        for i in range(y_block):
            for j in range(x_block):
                y1 = i * block_height
                y2 = (i+1) * block_height
                x1 = j * block_width
                x2 = (j+1) * block_width
                block = image[y1:y2, x1:x2]
                # Choose a random gradient (Sobel or Laplacian)
                gradient = random.choice(['sobel', 'laplacian'])
                if gradient == 'sobel':
                    block = cv2.Sobel(block, cv2.CV_64F, 1, 0, ksize=5)
                elif gradient == 'laplacian':
                    block = cv2.Laplacian(block, cv2.CV_64F, ksize=5)
                # Scale the gradient image back to 8-bit unsigned integers
                block = cv2.normalize(block, None, 0, 255, cv2.NORM_MINMAX, cv2.CV_8U)
                new_image[y1:y2, x1:x2] = block   
    else:
        raise ValueError("Invalid scramble type. Must be either 'classic' or 'pixel'.")
    
    
    if write == True:
        img_name = os.path.splitext(os.path.basename(image_path))[0]

        cv2.imwrite(f'{img_name}SCRAMBLED_' + f'{x_block, y_block}.png', new_image)
    else:
        return new_image


def scramblenoiseblur(img,cylce=5,kernel=(3,3),sigma=10):
    """scramblepix: Scramble the pixels of an image.
    Args:
        img: input image
        cylce: number of times to scramble the image
        kernel: kernel size for gaussian blur
        sigma: sigma for gaussian blur
    Usage:
        you need to import numpy and cv2
        img=cv2.imread("image.jpg")
        scramblepix(img,10,(3,3),10)
    """
    for i in range(cylce):
        img=img+np.random.normal(0,1,(img.shape[0],img.shape[1],3))*255
        gblur=cv2.GaussianBlur(img,kernel,sigma)
        img=gblur
    return img


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


