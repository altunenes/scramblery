#collection of functions for scrambling
import mediapipe as mp
import cv2
import numpy as np

mp_face_mesh = mp.solutions.face_mesh
face_mesh = mp_face_mesh.FaceMesh()


def get_facial_landmarks(frame):
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




def scrambleface(img,splits):
    """"scramble_face: Scramble the facial area of image.
    Args:
        img: input image
        splits: number of splits to perform
    Returns:
        img: output image

    usage:
        scrambleface("image.png",10)
    """

    img = cv2.imread(img)

    h, w, _ = img.shape
    mask = np.zeros((h, w), np.uint8)
    landmarks = get_facial_landmarks(img)
    hull = cv2.convexHull(landmarks)
    cv2.fillConvexPoly(mask, hull, 255)

    cv2.polylines(img, [hull], 1, (0, 0, 255), 1)

    cv2.polylines(mask, [hull], 1, (255), 1)

    for i in range(0, 468):
        pt = landmarks[i]

    splits=int(splits)
    img_new = np.array_split(img, splits)
    np.random.shuffle(img_new)

    img_new = np.vstack(img_new)

    face = cv2.bitwise_and(img_new, img_new, mask=mask)
    # Extract the face
    frame_copy = img_new
    face_extracted = cv2.bitwise_and(frame_copy, frame_copy, mask=mask)

    backgroundm = cv2.bitwise_not(mask)
    background = cv2.bitwise_and(img, img, mask=backgroundm)

    result = cv2.add(background, face_extracted)
    cv2.imwrite("scrambledImageface.png", result)
    print("scrambledImageface.png saved")


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
    image = cv2.imread(image_path)
    new_image = scramble_image_data(image, x_block, y_block)
    image_name = image_path.split('/')[-1]
    image_name = image_name.split('.')[0]
    print("your image has been scrambled with dimensions",x_block,"x",y_block)
    cv2.imwrite(f'{image_name}SCRAMBLED_' + f'{x_block, y_block}.png', new_image)



def scrambleimage(image,x_block,y_block):
    """scramble_image: Scramble the whole image.
    Args:
        image: input image
        x_block: number of splits in x-axis
        y_block: number of splits in y-axis

        Usage:
            scrambleimage("image.png",10,10)
    """
    x_block=int(x_block)
    y_block=int(y_block)
    scramble_image_data2(image,x_block,y_block)

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

        # frame_copy = cv2.blur(frame_copy, (27, 27))
        img_new = np.array_split(frame_copy, splits)
        np.random.shuffle(img_new)

        img_new = np.vstack(img_new)

        #face = cv2.bitwise_and(img_new, img_new, mask=mask)
        # Extract the face
        frame_copy = img_new
        face_extracted = cv2.bitwise_and(frame_copy, frame_copy, mask=mask)

        backgroundm = cv2.bitwise_not(mask)
        background = cv2.bitwise_and(frame, frame, mask=backgroundm)

        result = cv2.add(background, face_extracted)

        cv2.imshow("mask", result)

        cv2.waitKey(1)


