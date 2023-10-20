#############################      SCRAMBLERY FUNCTIONS IN PYTHON   #####################################
#############################      MAINTAINED BY: @altunenes        #####################################

import mediapipe as mp
import cv2
import numpy as np
import os
import random
from numpy.fft import fft2, ifft2, fftshift, ifftshift

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
def scrambleface(img, splits, type, seamless=False, bg=True, seed=None, write=True, scramble_ratio=1.0):
    """
    Scramble the facial area of the image.

    Args:
        img: input image path or image array
        splits: number of splits to perform, works differently between square and stack splits
        type: type of split, "pixel", "stack", or "fourier". Stack works with pixel coordinates, square works with pixel values
        seamless: if True, uses seamlessClone to blend the scrambled face with the original image
        bg: if True, uses the background of the original image, if False, replaces the background with a gray color
        seed: seed for random number generator (default: None)
        write: if True, writes the output image to disk, if False, returns the output image
        scramble_ratio: the power of Fourier scrambling (default: 1.0)

    Usage:
        scrambleface("image.png", 10, "fourier", False, True, None, True, 0.5)
    """
    img_name = None

    if seed is not None:
        np.random.seed(seed)

    if type not in ["stack", "pixel", "fourier"]:
        raise ValueError("type must be 'stack', 'pixel', or 'fourier'")
    
    if bg == False and seamless == True:
        print("You can't have seamless without bg")
        seamless = False

    if isinstance(img, str):
        image_path = img
        img = cv2.imread(img)
        if img is None:
            raise ValueError("Could not read the image")

        img_name = os.path.splitext(os.path.basename(image_path))[0]



    img_copy = img.copy()
    h, w, _ = img.shape
    mask = np.zeros((h, w), np.uint8)
    landmarks = get_facial_landmarks(img)
    hull = cv2.convexHull(landmarks)
    cv2.fillConvexPoly(mask, hull, 255)

    if type == "fourier":
        gray_image_full = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)

        face_region = cv2.bitwise_and(gray_image_full, gray_image_full, mask=mask)

        scrambled_face = fourier_scramble(face_region, scramble_ratio=scramble_ratio)

        if bg:
            output = gray_image_full.copy() 
        else:
            output = 128 * np.ones_like(gray_image_full)

        output[mask == 255] = scrambled_face[mask == 255]


        if seamless:
            M = cv2.moments(hull)
            cX = int(M["m10"] / M["m00"])
            cY = int(M["m01"] / M["m00"])
            center = (cX, cY)
            output = cv2.seamlessClone(output, img_copy, mask, center, cv2.NORMAL_CLONE)

        if write:
            filename_suffix = f"SCRAMBLED_fourier_{scramble_ratio}" + ("_seamless" if seamless else "") + ("" if bg else "_nobg") + ".png"
            cv2.imwrite(f'{img_name}{filename_suffix}', output)
        else:
            return output

    elif type == "stack":
        splits = int(splits)
        img_new = np.array_split(img, splits)
        np.random.shuffle(img_new)
        img_new = np.vstack(img_new)
        face = cv2.bitwise_and(img_new, img_new, mask=mask)

        face_only = 128 * np.ones_like(img)
        face_only[mask == 255] = face[mask == 255]

        frame_copy = img_new
        face_extracted = cv2.bitwise_and(frame_copy, frame_copy, mask=mask)

        backgroundm = cv2.bitwise_not(mask)
        background = cv2.bitwise_and(img, img, mask=backgroundm)

        result = cv2.add(background, face_extracted)

        if seamless:
            M = cv2.moments(hull)
            cX = int(M["m10"] / M["m00"])
            cY = int(M["m01"] / M["m00"])
            center = (cX, cY)
            result = cv2.seamlessClone(result, img_copy, mask, center, cv2.NORMAL_CLONE)
            if write:
                cv2.imwrite(f'{img_name}_SCRAMBLED_seamless_{splits}.png', result)
            else:
                return result
        elif bg == False:
            if write:
                cv2.imwrite(f'{img_name}_SCRAMBLED_nobg_{splits}.png', face_only)
            else:
                return face_only
        else:
            img_copy[mask == 255] = result[mask == 255]
            if write:
                cv2.imwrite(f'{img_name}_SCRAMBLED_{splits}.png', img_copy)
            else:
                return img_copy

    elif type == "pixel":
        for i in range(0, splits):
            for j in range(0, splits):
                x1 = int((i / splits) * w)
                y1 = int((j / splits) * h)
                x2 = int(((i + 1) / splits) * w)
                y2 = int(((j + 1) / splits) * h)

                img[y1:y2, x1:x2] = img[np.random.randint(y1, y2), np.random.randint(x1, x2)]

        out = 128 * np.ones_like(img)
        out[mask == 255] = img[mask == 255]

        if seamless:
            M = cv2.moments(hull)
            cX = int(M["m10"] / M["m00"])
            cY = int(M["m01"] / M["m00"])
            center = (cX, cY)
            out = cv2.seamlessClone(out, img_copy, mask, center, cv2.NORMAL_CLONE)
            if write:
                cv2.imwrite(f'{img_name}_SCRAMBLED_seamless_{splits}_{splits}.png', out)
            else:
                return out
        elif bg == False:
            if write:
                cv2.imwrite(f'{img_name}_SCRAMBLED_nobg_{splits}_{splits}.png', out)
            else:
                return out
        else:
            img_copy[mask == 255] = out[mask == 255]
            if write:
                cv2.imwrite(f'{img_name}_SCRAMBLED_{splits}_{splits}.png', img_copy)
            else:
                return img_copy    


def scrambleimage(image_path, x_block=10, y_block=10, scramble_type='classic', seed=None, write=True, **kwargs):
    """
    Main function to scramble an image based on the specified scramble type.

    Args:
        image_path: Path to the input image (with extension).
        x_block: Number of splits in x-axis.
        y_block: Number of splits in y-axis.
        scramble_type: Type of split; valid values are "classic", "pixel", "withinblocks", "rotate", "colormap", "gradient", "fourier".
        seed: Seed for random number generator (default: None).
        write: Write the image to disk (default: True).
        **kwargs: Additional keyword arguments specific to scramble types.

    Returns:
        If write is True, writes the scrambled image to disk.
        If write is False, returns the scrambled image.

    Raises:
        ValueError: If an invalid scramble_type is provided or image cannot be read.
        FileNotFoundError: If the image_path does not exist.
    """
    # Validation
    if not os.path.exists(image_path):
        raise FileNotFoundError(f"No such file: '{image_path}'")

    if scramble_type not in ["classic", "pixel", "withinblocks", "rotate", "colormap", "gradient", "fourier"]:
        raise ValueError(f"Invalid scramble type: {scramble_type}")

    if seed is not None:
        np.random.seed(seed)

    image = cv2.imread(image_path)
    if image is None:
        raise ValueError(f"Could not read the image: '{image_path}'")

    # A dictionary linking scramble types to their respective functions
    scramble_functions = {
        'classic': classic_scramble,
        'pixel': pixel_scramble,
        'withinblocks': withinblocks_scramble,
        'rotate': rotate_scramble,
        'colormap': colormap_scramble,
        'gradient': gradient_scramble,
        'fourier': fourier_scramble  # this function will be called differently
    }

    scramble_function = scramble_functions[scramble_type]

    if scramble_type == 'fourier':
        scrambled_image = scramble_function(image, **kwargs)
    else:
        scrambled_image = scramble_function(image, x_block, y_block, **kwargs)

    if write:
        if scramble_type == 'fourier':
            scramble_ratio = kwargs.get('scramble_ratio', 1.0)  # default to 1.0 if not provided
            output_filename = f"{os.path.splitext(image_path)[0]}_SCRAMBLED_fourier_{scramble_ratio}.png"
        else:
            output_filename = f"{os.path.splitext(image_path)[0]}_SCRAMBLED_{x_block}_{y_block}.png"

        cv2.imwrite(output_filename, scrambled_image)
        print(f"Image written to {output_filename}")
    else:
        return scrambled_image
def classic_scramble(image, x_block, y_block):
    """
    Scrambles an image by shuffling its blocks.

    Args:
        image: The image to scramble.
        x_block: Number of splits in x-axis.
        y_block: Number of splits in y-axis.

    Returns:
        The scrambled image.
    """
    h, w, _ = image.shape
    block_width = w // x_block
    block_height = h // y_block
    blocks = [image[i * block_height:(i + 1) * block_height, j * block_width:(j + 1) * block_width] for i in range(y_block) for j in range(x_block)]
    np.random.shuffle(blocks)

    output = np.zeros_like(image)

    for i in range(y_block):
        for j in range(x_block):
            output[i * block_height:(i + 1) * block_height, j * block_width:(j + 1) * block_width] = blocks[i * x_block + j]

    return output

def pixel_scramble(image, x_block, y_block):
    """
    Scrambles an image by randomly assigning pixel colors within blocks.

    Args:
        image: The image to scramble.
        x_block: Number of splits in x-axis.
        y_block: Number of splits in y-axis.

    Returns:
        The scrambled image.
    """
    h, w, _ = image.shape
    new_image = np.copy(image)
    for i in range(y_block):
        for j in range(x_block):
            y1 = i * (h // y_block)
            y2 = (i + 1) * (h // y_block)
            x1 = j * (w // x_block)
            x2 = (j + 1) * (w // x_block)

            rand_y = random.randint(y1, y2-1)
            rand_x = random.randint(x1, x2-1)
            new_image[y1:y2, x1:x2] = image[rand_y, rand_x]

    return new_image
def withinblocks_scramble(image, x_block, y_block):
    """
    Scrambles an image by shuffling pixels within each block.

    Args:
        image: The image to scramble.
        x_block: Number of splits in x-axis.
        y_block: Number of splits in y-axis.

    Returns:
        The scrambled image.
    """
    h, w, _ = image.shape
    new_image = np.zeros_like(image)

    for i in range(y_block):
        for j in range(x_block):
            y1 = i * (h // y_block)
            y2 = (i + 1) * (h // y_block)
            x1 = j * (w // x_block)
            x2 = (j + 1) * (w // x_block)
            block = image[y1:y2, x1:x2]

            block = block.reshape(-1, 3)
            np.random.shuffle(block)
            block = block.reshape((y2 - y1, x2 - x1, 3))

            new_image[y1:y2, x1:x2] = block

    return new_image
def rotate_scramble(image, x_block, y_block):
    """
    Scrambles an image by rotating each block by a random angle.

    Args:
        image: The image to scramble.
        x_block: Number of splits in x-axis.
        y_block: Number of splits in y-axis.

    Returns:
        The scrambled image.
    """
    h, w, _ = image.shape
    new_image = np.zeros_like(image)

    for i in range(y_block):
        for j in range(x_block):
            y1 = i * (h // y_block)
            y2 = (i + 1) * (h // y_block)
            x1 = j * (w // x_block)
            x2 = (j + 1) * (w // x_block)
            block = image[y1:y2, x1:x2]

            center = ((x2 - x1) / 2, (y2 - y1) / 2)
            angle = random.uniform(-45, 45)

            M = cv2.getRotationMatrix2D(center, angle, 1.0)
            rotated_block = cv2.warpAffine(block, M, (x2 - x1, y2 - y1))

            new_image[y1:y2, x1:x2] = rotated_block

    return new_image
def colormap_scramble(image, x_block, y_block):
    """
    Scrambles an image by applying a random colormap to each block.

    Args:
        image: The image to scramble.
        x_block: Number of splits in x-axis.
        y_block: Number of splits in y-axis.

    Returns:
        The scrambled image.
    """
    h, w, _ = image.shape
    new_image = np.zeros_like(image)

    colormaps = [
        cv2.COLORMAP_AUTUMN, cv2.COLORMAP_BONE, cv2.COLORMAP_JET,
        cv2.COLORMAP_WINTER, cv2.COLORMAP_RAINBOW, cv2.COLORMAP_OCEAN
    ]

    for i in range(y_block):
        for j in range(x_block):
            y1 = i * (h // y_block)
            y2 = (i + 1) * (h // y_block)
            x1 = j * (w // x_block)
            x2 = (j + 1) * (w // x_block)
            block = image[y1:y2, x1:x2]
            colormap = random.choice(colormaps)
            colored_block = cv2.applyColorMap(block, colormap)

            new_image[y1:y2, x1:x2] = colored_block

    return new_image
def gradient_scramble(image, x_block, y_block):
    """
    Scrambles an image by applying a random gradient effect to each block.

    Args:
        image: The image to scramble.
        x_block: Number of splits in x-axis.
        y_block: Number of splits in y-axis.

    Returns:
        The scrambled image.
    """
    h, w, _ = image.shape
    new_image = np.zeros_like(image)

    for i in range(y_block):
        for j in range(x_block):
            y1 = i * (h // y_block)
            y2 = (i + 1) * (h // y_block)
            x1 = j * (w // x_block)
            x2 = (j + 1) * (w // x_block)
            block = image[y1:y2, x1:x2]
            gradient_type = random.choice(['sobel', 'laplacian'])
            if gradient_type == 'sobel':
                gx = cv2.Sobel(block, cv2.CV_64F, 1, 0, ksize=5)
                gy = cv2.Sobel(block, cv2.CV_64F, 0, 1, ksize=5)
                block = cv2.magnitude(gx, gy)
            elif gradient_type == 'laplacian':
                block = cv2.Laplacian(block, cv2.CV_64F)
            block = cv2.normalize(block, None, 0, 255, cv2.NORM_MINMAX, cv2.CV_8U)

            new_image[y1:y2, x1:x2] = block

    return new_image
def fourier_scramble(image, scramble_ratio=1.0, **kwargs):
    """
    Scrambles an image by randomizing the phase in the frequency domain (Fourier domain),
    while keeping the magnitude constant. The amount of phase randomization is controlled
    by the scramble_ratio.

    Args:
        image: The image to scramble.
        scramble_ratio: The ratio of phase scrambling (default is 1.0, which means complete scrambling).
        **kwargs: Additional keyword arguments that are ignored in this function.

    Returns:
        The phase-scrambled image.
    """
    scramble_ratio = kwargs.get('scramble_ratio', 1.0)  # default to 1.0 if not provided
    if image.ndim == 2:  # The image is already grayscale
        gray_image = image
    elif image.ndim == 3: 
        gray_image = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
    else:
        raise ValueError("The input image should be a 2D grayscale or a 3D color image.")
    
    if not (0 <= scramble_ratio <= 1):
        raise ValueError("scramble_ratio must be between 0 and 1")

    f_transform = fft2(gray_image)

    f_transform_shift = fftshift(f_transform)

    magnitude = np.abs(f_transform_shift)
    phase = np.angle(f_transform_shift)

    random_phase = np.exp(1j * (2 * np.pi * np.random.rand(*phase.shape) - np.pi))

    new_phase = (1 - scramble_ratio) * phase + scramble_ratio * np.angle(random_phase)

    new_transform = magnitude * np.exp(1j * new_phase)

    f_ishift = ifftshift(new_transform)
    img_back = ifft2(f_ishift)

    img_back = np.real(img_back)

    img_back = (img_back - np.min(img_back)) / (np.max(img_back) - np.min(img_back))
    final_image = (255 * img_back).astype(np.uint8)

    return final_image
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


def scramblevideo(input_video_path, output_video_path=None, scramble_settings=None):
    """
    Scramble the facial area in a video.
    
    Args:
        input_video_path: Path to the input video.
        splits: Number of splits to perform on the face.
        output_video_path: Path where the output video will be saved (if None, the video won't be saved).
        scramble_settings: Dictionary of settings for the scrambleface function.
    
    Usage:
        scramblevideo("input_video.mp4", 10, "output_video.mp4", scramble_settings)
    """

    # Check if scramble_settings is provided, else use default settings
    if scramble_settings is None:
        scramble_settings = {
            'splits': 25,
            'type': 'pixel',
            'seamless': False,
            'bg': True,
            'seed': None,
            'write': False  # Important: this should always be False for video processing
        }
    else:
        if "write" in scramble_settings and scramble_settings["write"]:
            print("Warning: The 'write' setting in scramble_settings must be False for video processing. Overriding it to False.")
            scramble_settings["write"] = False

    # Open the video file
    cap = cv2.VideoCapture(input_video_path)
    if not cap.isOpened():
        print(f"Error: Could not open video {input_video_path}.")
        return

    frame_width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    frame_height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
    fps = cap.get(cv2.CAP_PROP_FPS)
    codec = cv2.VideoWriter_fourcc(*'XVID')
    total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))

    if output_video_path:
        out = cv2.VideoWriter(output_video_path, codec, fps, (frame_width, frame_height))
    else:
        out = None
        print("No output path specified. The scrambled video won't be saved.")

    frame_number = 0
    while cap.isOpened():
        ret, frame = cap.read()
        if not ret:
            print("Reached the end of the video.")
            break

        frame_number += 1
        print(f"Processing frame {frame_number}/{total_frames}")

        try:
            scrambled_frame = scrambleface(frame, **scramble_settings)
        except Exception as e:
            print(f"An error occurred while processing frame {frame_number}: {str(e)}")
            scrambled_frame = frame  

        if out:
            out.write(scrambled_frame)

    cap.release()
    if out:
        out.release()

    cv2.destroyAllWindows()
    print("Video processing completed.")


