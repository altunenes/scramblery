(function(){function r(e,n,t){function o(i,f){if(!n[i]){if(!e[i]){var c="function"==typeof require&&require;if(!f&&c)return c(i,!0);if(u)return u(i,!0);var a=new Error("Cannot find module '"+i+"'");throw a.code="MODULE_NOT_FOUND",a}var p=n[i]={exports:{}};e[i][0].call(p.exports,function(r){var n=e[i][1][r];return o(n||r)},p,p.exports,r,e,n,t)}return n[i].exports}for(var u="function"==typeof require&&require,i=0;i<t.length;i++)o(t[i]);return o}return r})()({1:[function(require,module,exports){
document.addEventListener("DOMContentLoaded", function() {
  let originalImage; 
  let processedImage; 
  document.body.style.fontFamily = 'sans-serif';
  document.body.style.margin = '0';
  document.body.style.padding = '0';
  document.body.style.display = 'flex';
  document.body.style.flexDirection = 'column';
  document.body.style.alignItems = 'center';
  document.body.style.justifyContent = 'flex-start';
  document.body.style.overflow = 'auto';
  const headerContainer = document.createElement('div');
  headerContainer.style.display = 'flex';
  headerContainer.style.alignItems = 'center';
  headerContainer.style.justifyContent = 'space-between';
  headerContainer.style.width = '100%';
  headerContainer.style.backgroundColor = '#6200ee';
  
  const spacer = document.createElement('div');
  spacer.style.width = '150px'; 
  spacer.style.flexShrink = '0'; 
  headerContainer.appendChild(spacer);
  
  const header = document.createElement('h1');
  header.textContent = 'Scramblery';
  header.style.textAlign = 'center';
  header.style.margin = '0';
  header.style.padding = '10px 0';
  header.style.flexGrow = '1'; 
  header.style.color = 'White'; 

  headerContainer.appendChild(header);
  
  const githubLink = document.createElement('a');
  githubLink.href = 'https://github.com/altunenes/scramblery';
  githubLink.textContent = 'View on GitHub';
  githubLink.style.color = 'white';
  githubLink.style.marginRight = '20px';
  githubLink.style.textDecoration = 'none';
  githubLink.style.fontSize = '16px';
  githubLink.style.border = '1px solid white';
  githubLink.style.padding = '5px 10px';
  githubLink.style.borderRadius = '5px';
  githubLink.style.transition = 'background-color 0.3s';
  githubLink.addEventListener('mouseenter', () => {
      githubLink.style.backgroundColor = 'rgba(255, 255, 255, 0.2)';
  });
  githubLink.addEventListener('mouseleave', () => {
      githubLink.style.backgroundColor = 'transparent';
  });
  headerContainer.appendChild(githubLink);
  
  document.body.insertBefore(headerContainer, document.body.firstChild);

  const controlsContainer = document.createElement('div');
  controlsContainer.style.display = 'flex';
  controlsContainer.style.flexDirection = 'row';
  controlsContainer.style.flexWrap = 'wrap';
  controlsContainer.style.justifyContent = 'center';
  controlsContainer.style.gap = '10px';
  controlsContainer.style.margin = '20px 0';
  document.body.appendChild(controlsContainer);

  function createButton(text, onClick) {
      const button = document.createElement('button');
      button.innerHTML = text;
      button.onclick = onClick;
      button.style.fontSize = '20px';
      button.style.padding = '10px';
      button.style.margin = '0';
      button.style.borderRadius = '10px';
      button.style.border = 'none';
      button.style.backgroundColor = '#7c4dff';
      button.style.color = 'white';
      button.style.cursor = 'pointer';
      controlsContainer.appendChild(button);
      return button;
  }

  const imagesContainer = document.createElement('div');
  imagesContainer.style.display = 'flex';
  imagesContainer.style.flexDirection = 'row';
  imagesContainer.style.gap = '20px';
  imagesContainer.style.margin = '20px 0';
  imagesContainer.style.flexWrap = 'wrap';
  imagesContainer.style.justifyContent = 'center';
  document.body.appendChild(imagesContainer);

  function createImageContainer() {
      const container = document.createElement('div');
      container.style.border = '2px solid #6200ee';
      container.style.borderRadius = '10px';
      container.style.padding = '10px';
      container.style.width = '300px';
      container.style.height = '300px';
      container.style.display = 'flex';
      container.style.alignItems = 'center';
      container.style.justifyContent = 'center';
      container.style.overflow = 'hidden';
      imagesContainer.appendChild(container);
      return container;
  }

  const originalImageContainer = createImageContainer();
  const processedImageContainer = createImageContainer();

  createButton('Import Image', function() {
    const fileInput = document.createElement('input');
    fileInput.type = 'file';
    fileInput.onchange = function() {
        // Clear the existing image
        originalImageContainer.innerHTML = ''; 

        const file = fileInput.files[0];
        const reader = new FileReader();
        reader.onload = function(e) {
            originalImage = new Image();
            originalImage.src = e.target.result;

            const displayImage = document.createElement('img');
            displayImage.src = originalImage.src;
            displayImage.style.maxWidth = '100%';
            displayImage.style.maxHeight = '100%';
            originalImageContainer.appendChild(displayImage); 
        };
        reader.readAsDataURL(file);
    };
    fileInput.click();
});

const sliderContainer = document.createElement('div');
sliderContainer.style.display = 'flex';
sliderContainer.style.flexDirection = 'column';
sliderContainer.style.alignItems = 'center';
sliderContainer.style.margin = '20px';

const sliderValueLabel = document.createElement('span');
sliderValueLabel.innerHTML = 'Scramble Ratio: 1';
sliderValueLabel.style.marginBottom = '10px';
sliderValueLabel.style.fontSize = '18px';
sliderValueLabel.style.fontWeight = 'bold';

const slider = document.createElement('input');
slider.type = 'range';
slider.min = 1;
slider.max = 150;
slider.value = 1;
slider.id = 'mosaicSize';
slider.style.width = '300px';

slider.oninput = function() {
    sliderValueLabel.innerHTML = 'Scramble Ratio: ' + slider.value;
    console.log(slider.value); 
};

sliderContainer.appendChild(sliderValueLabel);
sliderContainer.appendChild(slider);
controlsContainer.appendChild(sliderContainer);
document.body.appendChild(sliderContainer);
  createButton('Noise', function() {
      if (!originalImage) return; 

      const canvas = document.createElement('canvas');
      canvas.width = originalImage.width;
      canvas.height = originalImage.height;
      const context = canvas.getContext('2d');
      context.drawImage(originalImage, 0, 0);

      const imageData = context.getImageData(0, 0, canvas.width, canvas.height);
      const data = imageData.data;
      for (let i = 0; i < data.length; i += 4) {
          if (Math.random() < slider.value / 100) {
              data[i] = Math.random() * 255;
              data[i + 1] = Math.random() * 255;
              data[i + 2] = Math.random() * 255;
          }
      }
      context.putImageData(imageData, 0, 0);

      createProcessedImage(canvas.toDataURL());
  });

  createButton('Tile Shuffle', function() {
      if (!originalImage) return; 

      const canvas = document.createElement('canvas');
      canvas.width = originalImage.width;
      canvas.height = originalImage.height;
      const context = canvas.getContext('2d');
      context.drawImage(originalImage, 0, 0);

      const imageData = context.getImageData(0, 0, canvas.width, canvas.height);
      const data = imageData.data;
      const tileWidth = Math.floor(canvas.width / slider.value);
      const tileHeight = Math.floor(canvas.height / slider.value);
      const tiles = [];

      for (let y = 0; y < canvas.height; y += tileHeight) {
          for (let x = 0; x < canvas.width; x += tileWidth) {
              const tile = context.getImageData(x, y, tileWidth, tileHeight);
              tiles.push(tile);
          }
      }

      tiles.sort(() => 0.5 - Math.random());

      let i = 0;
      for (let y = 0; y < canvas.height; y += tileHeight) {
          for (let x = 0; x < canvas.width; x += tileWidth) {
              context.putImageData(tiles[i++], x, y);
          }
      }

      createProcessedImage(canvas.toDataURL());
  });
  createButton('Pixel Scramble', function() {
    if (!originalImage) return; 

    const canvas = document.createElement('canvas');
    canvas.width = originalImage.width;
    canvas.height = originalImage.height;
    const ctx = canvas.getContext('2d');
    ctx.drawImage(originalImage, 0, 0);

    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    const data = imageData.data;
    const mosaicSize = parseInt(document.getElementById('mosaicSize').value);

    for (let i = 0; i < data.length; i += 4) {
        const x = (i / 4) % canvas.width;
        const y = Math.floor((i / 4) / canvas.width);
        if (x % mosaicSize === 0 && y % mosaicSize === 0) {
            for (let j = 0; j < mosaicSize; j++) {
                for (let k = 0; k < mosaicSize; k++) {
                    const index = (y + j) * canvas.width + (x + k);
                    data[index * 4] = data[i];
                    data[index * 4 + 1] = data[i + 1];
                    data[index * 4 + 2] = data[i + 2];
                    data[index * 4 + 3] = data[i + 3];
                }
            }
        }
    }
    ctx.putImageData(imageData, 0, 0);
    createProcessedImage(canvas.toDataURL());
});
function nextPowerOfTwo(x) {
    return Math.pow(2, Math.ceil(Math.log(x) / Math.log(2)));
}
function getPaddedCanvas(imageElement) {
    const canvas = document.createElement('canvas');
    canvas.width = nextPowerOfTwo(imageElement.naturalWidth);
    canvas.height = nextPowerOfTwo(imageElement.naturalHeight);
    const ctx = canvas.getContext('2d');
    
    // Calculate the scale to fit the image into the canvas, maintaining aspect ratio
    const scale = Math.min(canvas.width / imageElement.naturalWidth, canvas.height / imageElement.naturalHeight);
    const scaledWidth = imageElement.naturalWidth * scale;
    const scaledHeight = imageElement.naturalHeight * scale;
    
    // Calculate the offset to center the image in the canvas
    const offsetX = (canvas.width - scaledWidth) / 2;
    const offsetY = (canvas.height - scaledHeight) / 2;
    
    // Fill the canvas with a black background to prevent repeating patterns
    ctx.fillStyle = 'black';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    
    // Draw the image centered on the canvas
    ctx.drawImage(imageElement, offsetX, offsetY, scaledWidth, scaledHeight);
    
    return canvas;
}
createButton('Fourier Scramble', async function() {
    if (!originalImage) return;
    async function phaseScramble(imageElement) {
        await new Promise(resolve => {
            if (imageElement.complete) return resolve();
            imageElement.onload = () => resolve();
        });
        const canvas = getPaddedCanvas(imageElement);
        const ctx = canvas.getContext('2d');
        let imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
        const pixels = imageData.data;
        for (let i = 0; i < pixels.length; i += 4) {
            let lightness = parseInt((pixels[i] + pixels[i + 1] + pixels[i + 2]) / 3);
            pixels[i] = pixels[i + 1] = pixels[i + 2] = lightness;
        }
        ctx.putImageData(imageData, 0, 0);
        const FFT = require('fft.js');
        const fft = new FFT(canvas.width * canvas.height); // FFT size corrected
        const spatialDomain = Array.from({ length: canvas.width * canvas.height }, (_, i) => pixels[i * 4] / 255);
        const frequencyDomain = fft.createComplexArray();
        fft.realTransform(frequencyDomain, spatialDomain);
        const maxPhaseShift = parseFloat(slider.value) / 150 * 2 * Math.PI; 
        const halfLength = frequencyDomain.length / 2;
        for (let i = 0; i < halfLength; i += 2) {
            const amplitude = Math.sqrt(frequencyDomain[i] ** 2 + frequencyDomain[i + 1] ** 2);
            const phase = Math.atan2(frequencyDomain[i + 1], frequencyDomain[i]);
            const randomPhase = phase + (Math.random() - 0.5) * maxPhaseShift; 
            frequencyDomain[i] = amplitude * Math.cos(randomPhase); 
            frequencyDomain[i + 1] = amplitude * Math.sin(randomPhase); 
        }
        fft.completeSpectrum(frequencyDomain);
        const output = fft.createComplexArray();
        fft.inverseTransform(output, frequencyDomain);
        const realOutput = output.map((value, idx) => idx % 2 === 0 ? (value + 0.5) * 255 : value); 
        for (let i = 0; i < pixels.length; i += 4) {
            const newVal = realOutput[i / 2]; 
            pixels[i] = pixels[i + 1] = pixels[i + 2] = newVal;
        }
        ctx.putImageData(imageData, 0, 0);
        const finalCanvas = document.createElement('canvas');
        finalCanvas.width = imageElement.naturalWidth;
        finalCanvas.height = imageElement.naturalHeight;
        const finalCtx = finalCanvas.getContext('2d');
        finalCtx.drawImage(canvas, 0, 0, imageElement.naturalWidth, imageElement.naturalHeight);

        createProcessedImage(finalCanvas.toDataURL());
    }
    await phaseScramble(originalImage);
});
  createButton('Clear', function() {
      location.reload(); 
  });
  const downloadLink = document.createElement('a');
const downloadBtn = createButton('Download', function() {
    if (processedImage) {
        downloadLink.href = processedImage.src;
        downloadLink.download = 'processed_image.png';
        downloadLink.click();
    }
});

function createInfoModal() {
    const modal = document.createElement('div');
    const modalContent = document.createElement('div');
    const closeBtn = document.createElement('span');
    const infoHeader = document.createElement('h2');
    const infoParagraph = document.createElement('p');

    modal.style.display = 'none'; 
    modal.style.position = 'fixed';
    modal.style.zIndex = '1';
    modal.style.left = '0';
    modal.style.top = '0';
    modal.style.width = '100%';
    modal.style.height = '100%';
    modal.style.overflow = 'auto';
    modal.style.backgroundColor = 'rgba(0,0,0,0.4)';
    modal.style.backgroundColor = 'rgba(0,0,0,0.4)';

    modalContent.style.backgroundColor = '#fefefe';
    modalContent.style.margin = '15% auto';
    modalContent.style.padding = '20px';
    modalContent.style.border = '1px solid #888';
    modalContent.style.width = '70%'; 
    modalContent.style.boxShadow = '0 4px 8px 0 rgba(0,0,0,0.2), 0 6px 20px 0 rgba(0,0,0,0.19)';
    closeBtn.style.color = '#aaa';
    closeBtn.style.float = 'right';
    closeBtn.style.fontSize = '28px';
    closeBtn.style.fontWeight = 'bold';
    closeBtn.innerHTML = '&times;';
    closeBtn.style.cursor = 'pointer';

    closeBtn.onclick = function() {
        modal.style.display = 'none';
    }
    infoHeader.textContent = 'About Fourier Scrambling';
    infoParagraph.textContent = `Fourier Scrambling is a process that involves scrambling the phases of an image in the frequency domain. Each time you apply a Fourier Scramble, it randomly alters the phases, leading to a different result every time. It's important to note that this technique works best with square images, as non-square images can lead to distortions or repetitions due to the way the Fast Fourier Transform (FFT) operates.`;
    modalContent.appendChild(closeBtn);
    modalContent.appendChild(infoHeader);
    modalContent.appendChild(infoParagraph);
    modal.appendChild(modalContent);
    document.body.appendChild(modal);

    return modal;
}

function createAboutButton() {
    const aboutButton = document.createElement('button');
    aboutButton.innerHTML = 'About';
    aboutButton.onclick = function() {
        infoModal.style.display = 'block';
    };
    aboutButton.style.position = 'absolute';
    aboutButton.style.top = '10px';
    aboutButton.style.left = '20px';
    aboutButton.style.fontSize = '18px';
    aboutButton.style.padding = '10px';
    aboutButton.style.borderRadius = '5px';
    aboutButton.style.border = 'none';
    aboutButton.style.backgroundColor = '#6200ee';
    aboutButton.style.color = 'white';
    aboutButton.style.cursor = 'pointer';
    aboutButton.style.transition = 'background-color 0.3s, color 0.3s';
    aboutButton.addEventListener('mouseenter', () => {
        aboutButton.style.backgroundColor = 'white';
        aboutButton.style.color = '#6200ee'; 
    });
    aboutButton.addEventListener('mouseleave', () => {
        aboutButton.style.backgroundColor = '#6200ee';
        aboutButton.style.color = 'white';
    });
    document.body.appendChild(aboutButton);
}
const infoModal = createInfoModal();
createAboutButton();
controlsContainer.appendChild(downloadLink);
downloadLink.appendChild(downloadBtn);
  function createProcessedImage(dataUrl) {
      if (processedImage) {
          processedImageContainer.removeChild(processedImage);
      }
      processedImage = new Image();
      processedImage.src = dataUrl;
      processedImage.style.maxWidth = '100%';
      processedImage.style.maxHeight = '100%';
      processedImageContainer.appendChild(processedImage);
  }
});
},{"fft.js":3}],2:[function(require,module,exports){
// main.js
const FFT = require('fft.js');
require('./js/scramblery.js');

function testFFT() {
    const fft = new FFT(4); // using a small size for simplicity
    const input = [1, 2, 3, 4]; // example input signal
    const output = [];

    fft.realTransform(output, input);
    console.log(output); // should print the transformed signal
}

// Call testFFT when the window loads
window.onload = testFFT;
},{"./js/scramblery.js":1,"fft.js":3}],3:[function(require,module,exports){
'use strict';

function FFT(size) {
  this.size = size | 0;
  if (this.size <= 1 || (this.size & (this.size - 1)) !== 0)
    throw new Error('FFT size must be a power of two and bigger than 1');

  this._csize = size << 1;

  // NOTE: Use of `var` is intentional for old V8 versions
  var table = new Array(this.size * 2);
  for (var i = 0; i < table.length; i += 2) {
    const angle = Math.PI * i / this.size;
    table[i] = Math.cos(angle);
    table[i + 1] = -Math.sin(angle);
  }
  this.table = table;

  // Find size's power of two
  var power = 0;
  for (var t = 1; this.size > t; t <<= 1)
    power++;

  // Calculate initial step's width:
  //   * If we are full radix-4 - it is 2x smaller to give inital len=8
  //   * Otherwise it is the same as `power` to give len=4
  this._width = power % 2 === 0 ? power - 1 : power;

  // Pre-compute bit-reversal patterns
  this._bitrev = new Array(1 << this._width);
  for (var j = 0; j < this._bitrev.length; j++) {
    this._bitrev[j] = 0;
    for (var shift = 0; shift < this._width; shift += 2) {
      var revShift = this._width - shift - 2;
      this._bitrev[j] |= ((j >>> shift) & 3) << revShift;
    }
  }

  this._out = null;
  this._data = null;
  this._inv = 0;
}
module.exports = FFT;

FFT.prototype.fromComplexArray = function fromComplexArray(complex, storage) {
  var res = storage || new Array(complex.length >>> 1);
  for (var i = 0; i < complex.length; i += 2)
    res[i >>> 1] = complex[i];
  return res;
};

FFT.prototype.createComplexArray = function createComplexArray() {
  const res = new Array(this._csize);
  for (var i = 0; i < res.length; i++)
    res[i] = 0;
  return res;
};

FFT.prototype.toComplexArray = function toComplexArray(input, storage) {
  var res = storage || this.createComplexArray();
  for (var i = 0; i < res.length; i += 2) {
    res[i] = input[i >>> 1];
    res[i + 1] = 0;
  }
  return res;
};

FFT.prototype.completeSpectrum = function completeSpectrum(spectrum) {
  var size = this._csize;
  var half = size >>> 1;
  for (var i = 2; i < half; i += 2) {
    spectrum[size - i] = spectrum[i];
    spectrum[size - i + 1] = -spectrum[i + 1];
  }
};

FFT.prototype.transform = function transform(out, data) {
  if (out === data)
    throw new Error('Input and output buffers must be different');

  this._out = out;
  this._data = data;
  this._inv = 0;
  this._transform4();
  this._out = null;
  this._data = null;
};

FFT.prototype.realTransform = function realTransform(out, data) {
  if (out === data)
    throw new Error('Input and output buffers must be different');

  this._out = out;
  this._data = data;
  this._inv = 0;
  this._realTransform4();
  this._out = null;
  this._data = null;
};

FFT.prototype.inverseTransform = function inverseTransform(out, data) {
  if (out === data)
    throw new Error('Input and output buffers must be different');

  this._out = out;
  this._data = data;
  this._inv = 1;
  this._transform4();
  for (var i = 0; i < out.length; i++)
    out[i] /= this.size;
  this._out = null;
  this._data = null;
};

// radix-4 implementation
//
// NOTE: Uses of `var` are intentional for older V8 version that do not
// support both `let compound assignments` and `const phi`
FFT.prototype._transform4 = function _transform4() {
  var out = this._out;
  var size = this._csize;

  // Initial step (permute and transform)
  var width = this._width;
  var step = 1 << width;
  var len = (size / step) << 1;

  var outOff;
  var t;
  var bitrev = this._bitrev;
  if (len === 4) {
    for (outOff = 0, t = 0; outOff < size; outOff += len, t++) {
      const off = bitrev[t];
      this._singleTransform2(outOff, off, step);
    }
  } else {
    // len === 8
    for (outOff = 0, t = 0; outOff < size; outOff += len, t++) {
      const off = bitrev[t];
      this._singleTransform4(outOff, off, step);
    }
  }

  // Loop through steps in decreasing order
  var inv = this._inv ? -1 : 1;
  var table = this.table;
  for (step >>= 2; step >= 2; step >>= 2) {
    len = (size / step) << 1;
    var quarterLen = len >>> 2;

    // Loop through offsets in the data
    for (outOff = 0; outOff < size; outOff += len) {
      // Full case
      var limit = outOff + quarterLen;
      for (var i = outOff, k = 0; i < limit; i += 2, k += step) {
        const A = i;
        const B = A + quarterLen;
        const C = B + quarterLen;
        const D = C + quarterLen;

        // Original values
        const Ar = out[A];
        const Ai = out[A + 1];
        const Br = out[B];
        const Bi = out[B + 1];
        const Cr = out[C];
        const Ci = out[C + 1];
        const Dr = out[D];
        const Di = out[D + 1];

        // Middle values
        const MAr = Ar;
        const MAi = Ai;

        const tableBr = table[k];
        const tableBi = inv * table[k + 1];
        const MBr = Br * tableBr - Bi * tableBi;
        const MBi = Br * tableBi + Bi * tableBr;

        const tableCr = table[2 * k];
        const tableCi = inv * table[2 * k + 1];
        const MCr = Cr * tableCr - Ci * tableCi;
        const MCi = Cr * tableCi + Ci * tableCr;

        const tableDr = table[3 * k];
        const tableDi = inv * table[3 * k + 1];
        const MDr = Dr * tableDr - Di * tableDi;
        const MDi = Dr * tableDi + Di * tableDr;

        // Pre-Final values
        const T0r = MAr + MCr;
        const T0i = MAi + MCi;
        const T1r = MAr - MCr;
        const T1i = MAi - MCi;
        const T2r = MBr + MDr;
        const T2i = MBi + MDi;
        const T3r = inv * (MBr - MDr);
        const T3i = inv * (MBi - MDi);

        // Final values
        const FAr = T0r + T2r;
        const FAi = T0i + T2i;

        const FCr = T0r - T2r;
        const FCi = T0i - T2i;

        const FBr = T1r + T3i;
        const FBi = T1i - T3r;

        const FDr = T1r - T3i;
        const FDi = T1i + T3r;

        out[A] = FAr;
        out[A + 1] = FAi;
        out[B] = FBr;
        out[B + 1] = FBi;
        out[C] = FCr;
        out[C + 1] = FCi;
        out[D] = FDr;
        out[D + 1] = FDi;
      }
    }
  }
};

// radix-2 implementation
//
// NOTE: Only called for len=4
FFT.prototype._singleTransform2 = function _singleTransform2(outOff, off,
                                                             step) {
  const out = this._out;
  const data = this._data;

  const evenR = data[off];
  const evenI = data[off + 1];
  const oddR = data[off + step];
  const oddI = data[off + step + 1];

  const leftR = evenR + oddR;
  const leftI = evenI + oddI;
  const rightR = evenR - oddR;
  const rightI = evenI - oddI;

  out[outOff] = leftR;
  out[outOff + 1] = leftI;
  out[outOff + 2] = rightR;
  out[outOff + 3] = rightI;
};

// radix-4
//
// NOTE: Only called for len=8
FFT.prototype._singleTransform4 = function _singleTransform4(outOff, off,
                                                             step) {
  const out = this._out;
  const data = this._data;
  const inv = this._inv ? -1 : 1;
  const step2 = step * 2;
  const step3 = step * 3;

  // Original values
  const Ar = data[off];
  const Ai = data[off + 1];
  const Br = data[off + step];
  const Bi = data[off + step + 1];
  const Cr = data[off + step2];
  const Ci = data[off + step2 + 1];
  const Dr = data[off + step3];
  const Di = data[off + step3 + 1];

  // Pre-Final values
  const T0r = Ar + Cr;
  const T0i = Ai + Ci;
  const T1r = Ar - Cr;
  const T1i = Ai - Ci;
  const T2r = Br + Dr;
  const T2i = Bi + Di;
  const T3r = inv * (Br - Dr);
  const T3i = inv * (Bi - Di);

  // Final values
  const FAr = T0r + T2r;
  const FAi = T0i + T2i;

  const FBr = T1r + T3i;
  const FBi = T1i - T3r;

  const FCr = T0r - T2r;
  const FCi = T0i - T2i;

  const FDr = T1r - T3i;
  const FDi = T1i + T3r;

  out[outOff] = FAr;
  out[outOff + 1] = FAi;
  out[outOff + 2] = FBr;
  out[outOff + 3] = FBi;
  out[outOff + 4] = FCr;
  out[outOff + 5] = FCi;
  out[outOff + 6] = FDr;
  out[outOff + 7] = FDi;
};

// Real input radix-4 implementation
FFT.prototype._realTransform4 = function _realTransform4() {
  var out = this._out;
  var size = this._csize;

  // Initial step (permute and transform)
  var width = this._width;
  var step = 1 << width;
  var len = (size / step) << 1;

  var outOff;
  var t;
  var bitrev = this._bitrev;
  if (len === 4) {
    for (outOff = 0, t = 0; outOff < size; outOff += len, t++) {
      const off = bitrev[t];
      this._singleRealTransform2(outOff, off >>> 1, step >>> 1);
    }
  } else {
    // len === 8
    for (outOff = 0, t = 0; outOff < size; outOff += len, t++) {
      const off = bitrev[t];
      this._singleRealTransform4(outOff, off >>> 1, step >>> 1);
    }
  }

  // Loop through steps in decreasing order
  var inv = this._inv ? -1 : 1;
  var table = this.table;
  for (step >>= 2; step >= 2; step >>= 2) {
    len = (size / step) << 1;
    var halfLen = len >>> 1;
    var quarterLen = halfLen >>> 1;
    var hquarterLen = quarterLen >>> 1;

    // Loop through offsets in the data
    for (outOff = 0; outOff < size; outOff += len) {
      for (var i = 0, k = 0; i <= hquarterLen; i += 2, k += step) {
        var A = outOff + i;
        var B = A + quarterLen;
        var C = B + quarterLen;
        var D = C + quarterLen;

        // Original values
        var Ar = out[A];
        var Ai = out[A + 1];
        var Br = out[B];
        var Bi = out[B + 1];
        var Cr = out[C];
        var Ci = out[C + 1];
        var Dr = out[D];
        var Di = out[D + 1];

        // Middle values
        var MAr = Ar;
        var MAi = Ai;

        var tableBr = table[k];
        var tableBi = inv * table[k + 1];
        var MBr = Br * tableBr - Bi * tableBi;
        var MBi = Br * tableBi + Bi * tableBr;

        var tableCr = table[2 * k];
        var tableCi = inv * table[2 * k + 1];
        var MCr = Cr * tableCr - Ci * tableCi;
        var MCi = Cr * tableCi + Ci * tableCr;

        var tableDr = table[3 * k];
        var tableDi = inv * table[3 * k + 1];
        var MDr = Dr * tableDr - Di * tableDi;
        var MDi = Dr * tableDi + Di * tableDr;

        // Pre-Final values
        var T0r = MAr + MCr;
        var T0i = MAi + MCi;
        var T1r = MAr - MCr;
        var T1i = MAi - MCi;
        var T2r = MBr + MDr;
        var T2i = MBi + MDi;
        var T3r = inv * (MBr - MDr);
        var T3i = inv * (MBi - MDi);

        // Final values
        var FAr = T0r + T2r;
        var FAi = T0i + T2i;

        var FBr = T1r + T3i;
        var FBi = T1i - T3r;

        out[A] = FAr;
        out[A + 1] = FAi;
        out[B] = FBr;
        out[B + 1] = FBi;

        // Output final middle point
        if (i === 0) {
          var FCr = T0r - T2r;
          var FCi = T0i - T2i;
          out[C] = FCr;
          out[C + 1] = FCi;
          continue;
        }

        // Do not overwrite ourselves
        if (i === hquarterLen)
          continue;

        // In the flipped case:
        // MAi = -MAi
        // MBr=-MBi, MBi=-MBr
        // MCr=-MCr
        // MDr=MDi, MDi=MDr
        var ST0r = T1r;
        var ST0i = -T1i;
        var ST1r = T0r;
        var ST1i = -T0i;
        var ST2r = -inv * T3i;
        var ST2i = -inv * T3r;
        var ST3r = -inv * T2i;
        var ST3i = -inv * T2r;

        var SFAr = ST0r + ST2r;
        var SFAi = ST0i + ST2i;

        var SFBr = ST1r + ST3i;
        var SFBi = ST1i - ST3r;

        var SA = outOff + quarterLen - i;
        var SB = outOff + halfLen - i;

        out[SA] = SFAr;
        out[SA + 1] = SFAi;
        out[SB] = SFBr;
        out[SB + 1] = SFBi;
      }
    }
  }
};

// radix-2 implementation
//
// NOTE: Only called for len=4
FFT.prototype._singleRealTransform2 = function _singleRealTransform2(outOff,
                                                                     off,
                                                                     step) {
  const out = this._out;
  const data = this._data;

  const evenR = data[off];
  const oddR = data[off + step];

  const leftR = evenR + oddR;
  const rightR = evenR - oddR;

  out[outOff] = leftR;
  out[outOff + 1] = 0;
  out[outOff + 2] = rightR;
  out[outOff + 3] = 0;
};

// radix-4
//
// NOTE: Only called for len=8
FFT.prototype._singleRealTransform4 = function _singleRealTransform4(outOff,
                                                                     off,
                                                                     step) {
  const out = this._out;
  const data = this._data;
  const inv = this._inv ? -1 : 1;
  const step2 = step * 2;
  const step3 = step * 3;

  // Original values
  const Ar = data[off];
  const Br = data[off + step];
  const Cr = data[off + step2];
  const Dr = data[off + step3];

  // Pre-Final values
  const T0r = Ar + Cr;
  const T1r = Ar - Cr;
  const T2r = Br + Dr;
  const T3r = inv * (Br - Dr);

  // Final values
  const FAr = T0r + T2r;

  const FBr = T1r;
  const FBi = -T3r;

  const FCr = T0r - T2r;

  const FDr = T1r;
  const FDi = T3r;

  out[outOff] = FAr;
  out[outOff + 1] = 0;
  out[outOff + 2] = FBr;
  out[outOff + 3] = FBi;
  out[outOff + 4] = FCr;
  out[outOff + 5] = 0;
  out[outOff + 6] = FDr;
  out[outOff + 7] = FDi;
};

},{}]},{},[2]);
