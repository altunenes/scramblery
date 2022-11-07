 # Scramblery
[![Downloads](https://pepy.tech/badge/scramblery)](https://pepy.tech/project/scramblery)
[![PyPI version](https://badge.fury.io/py/scramblery.svg)](https://badge.fury.io/py/scramblery)
[![Jekyll site CI](https://github.com/altunenes/scramblery/actions/workflows/jekyll.yml/badge.svg)](https://github.com/altunenes/scramblery/actions/workflows/jekyll.yml)

A simple tool to scramble your images or only faces from images or videos. You can find the online demo in javascript [here](https://altunenes.github.io/scramblery/scramblerydemo.html). For more information, please visit the [documentation](https://altunenes.github.io/scramblery/).

#### Purpose of Package
 The purpose of this package is the creating scrambled images from images or videos. User can either scramble the whole image or only facial area.
 This is very useful tool in psychology experiments especially if you are working with faces. With a for loop you can scramble all the images in a folder and create a new folder with scrambled images. It was very long process to scramble images manually in the past and I feel like this package can be useful for many people. Hope this package will be useful for your research.

#### **Features**
- Scramble whole image with desired degree of scrambling (pixel values or pixel coordinates)
- Scramble only facial area with desired degree of scrambling (pixel values or pixel coordinates)
- Scramble only facial area in a video (useful for dynmaic stimuli) with desired degree of scrambling

#### Installation
- The package can be found in pypi. To install the package, run the following command in the terminal:
- `pip install scramblery`
#### Author

  -  Main Maintainer: [Enes ALTUN]


#### Usage
After installing the package, you can import the package as follows:
- `from scramblery import scramblery`
Then use the functions as follows to scramble images. I added some examples below.

  ![8x8](/docs/assets/usage.png)

### Contributon
 Any kind of contribution is welcome.