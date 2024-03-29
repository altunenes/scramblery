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

    // Set the scale to maintain aspect ratio based on the larger dimension
    const scale = Math.min(canvas.width / imageElement.naturalWidth, canvas.height / imageElement.naturalHeight);
    const scaledWidth = imageElement.naturalWidth * scale;
    const scaledHeight = imageElement.naturalHeight * scale;
    
    // Calculate the offset to center the image on the canvas
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