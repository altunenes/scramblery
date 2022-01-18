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
cap=cv2.VideoCapture("sadsdsa.mp4")

while True:

    ret,frame=cap.read()
    frame_copy=frame.copy()
    h,w,_=frame.shape
    mask=np.zeros((h,w),np.uint8)
    landmarks=get_facial_landmarks(frame)
    print(landmarks)
    hull=cv2.convexHull(landmarks)
    cv2.fillConvexPoly(mask, hull, 255)

    cv2.polylines(frame,[hull],1,(0,0,255),1)

    cv2.polylines(mask,[hull],1,(255),1)



    for i in range(0,468):
        pt=landmarks[i]
    if ret is not True:
        break


    # frame_copy = cv2.blur(frame_copy, (27, 27))
    img_new = np.array_split(frame_copy, 77)
    np.random.shuffle(img_new)

    img_new = np.vstack(img_new)

    face=cv2.bitwise_and(img_new,img_new,mask=mask)
    # Extract the face
    frame_copy = img_new
    face_extracted = cv2.bitwise_and(frame_copy, frame_copy, mask=mask)


    backgroundm=cv2.bitwise_not(mask)
    background = cv2.bitwise_and(frame, frame, mask=backgroundm)

    result = cv2.add(background, face_extracted)

    cv2.imshow("mask",result)

    cv2.waitKey(1)
