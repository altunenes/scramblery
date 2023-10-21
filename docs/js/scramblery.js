let isCvReady = false;
function onOpenCvReady() {
    isCvReady = true;
    document.getElementById('status').innerHTML = 'OpenCV.js is ready.';
}
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
  const header = document.createElement('h1');
  header.textContent = 'Scramblery';
  header.style.textAlign = 'center';
  header.style.width = '100%';
  header.style.backgroundColor = '#6200ee';
  header.style.color = 'white';
  header.style.margin = '0';
  header.style.padding = '10px 0';
  document.body.appendChild(header);

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

  const slider = document.createElement('input');
  slider.type = 'range';
  slider.min = 1;
  slider.max = 150;
  slider.value = 1;
  slider.id = 'mosaicSize';
  slider.oninput = function() {
      console.log(slider.value);
  };
  controlsContainer.appendChild(slider);

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

  createButton('Scramble Image', function() {
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

  createButton('Clear', function() {
      location.reload(); 
  });

  createButton('Scramble Mosaic', function() {
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
  const statusIndicator = document.createElement('div');
  statusIndicator.id = 'status';
  statusIndicator.textContent = 'Loading OpenCV.js...';
  document.body.appendChild(statusIndicator);


  
  createButton('Fourier Scramble', function() {
    if (!originalImage || !isCvReady) {
      alert('Please wait for OpenCV to load, or make sure an image is loaded.');
      return;
    }

    const canvas = document.createElement('canvas');
    canvas.width = originalImage.width;
    canvas.height = originalImage.height;
    const context = canvas.getContext('2d');
    context.drawImage(originalImage, 0, 0);

    let imageData = context.getImageData(0, 0, canvas.width, canvas.height);

    let gray = cv.imread(imageData);
    cv.cvtColor(gray, gray, cv.COLOR_RGBA2GRAY);

    let scrambledImage = fourierTransformAndScramble(gray);
    
    let resultImageData = new ImageData(new Uint8ClampedArray(scrambledImage.data), gray.cols, gray.rows);

    context.putImageData(resultImageData, 0, 0);
    createProcessedImage(canvas.toDataURL());

    gray.delete(); scrambledImage.delete();
});

function fourierTransformAndScramble(mat) {
    const optimalRows = cv.getOptimalDFTSize(mat.rows);
    const optimalCols = cv.getOptimalDFTSize(mat.cols);
    let padded = new cv.Mat();
    cv.copyMakeBorder(mat, padded, 0, optimalRows - mat.rows, 0, optimalCols - mat.cols, cv.BORDER_CONSTANT, new cv.Scalar());

    padded.convertTo(padded, cv.CV_32F);

    let planes = new cv.MatVector();
    planes.push_back(padded);
    planes.push_back(new cv.Mat.zeros(padded.rows, padded.cols, cv.CV_32F));
    let complexI = new cv.Mat();
    cv.merge(planes, complexI);
    cv.dft(complexI, complexI);

    let mag = new cv.Mat(), angle = new cv.Mat();
    cv.cartToPolar(planes.get(0), planes.get(1), mag, angle, false);
    let randomPhase = new cv.Mat(angle.rows, angle.cols, cv.CV_32F, new cv.Scalar(Math.PI));
    cv.addWeighted(angle, 1, randomPhase, 1, 0, angle, -1);
    cv.polarToCart(mag, angle, planes.get(0), planes.get(1), false);

    cv.merge(planes, complexI);
    cv.idft(complexI, complexI, cv.DFT_REAL_OUTPUT);
    let restored = new cv.Mat();
    cv.split(complexI, planes);
    planes.get(0).convertTo(restored, cv.CV_8U);

    padded.delete(); planes.delete(); complexI.delete(); mag.delete(); angle.delete(); randomPhase.delete();

    return restored;
}
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


