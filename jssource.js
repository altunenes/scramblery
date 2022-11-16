  /*/  The web version of Scramblery, is still under construction  /*/  
  var importImageButton = document.createElement('button');
    importImageButton.innerHTML = 'Import Image';
    /*/importImageButton.style.position = 'fixed';/*/
    importImageButton.onclick = function() {
    var fileInput = document.createElement('input');
    fileInput.type = 'file';
    fileInput.onchange = function() {
    var file = fileInput.files[0];
    var reader = new FileReader();
    reader.onload = function(e) {
    var image = document.createElement('img');
    image.src = e.target.result;
    document.body.appendChild(image);
    };
    reader.readAsDataURL(file);
    };
    fileInput.click();
    };
    document.body.appendChild(importImageButton);
    
    
    
    
    
    var slider = document.createElement('input');
    slider.type = 'range';
    slider.min = 1;
    slider.max = 150;
    slider.value = 1;
    slider.oninput = function() {
    console.log(slider.value);
    };
    document.body.appendChild(slider);
    var gaussianNoiseButton = document.createElement('button');
    gaussianNoiseButton.innerHTML = 'Noise';
    gaussianNoiseButton.onclick = function() {
    var image = document.querySelector('img');
    var canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    var context = canvas.getContext('2d');
    context.drawImage(image, 0, 0);
    var imageData = context.getImageData(0, 0, canvas.width, canvas.height);
    var data = imageData.data;
    for (var i = 0; i < data.length; i += 4) {
    if (Math.random() < slider.value / 100) {
    data[i] = Math.random() * 255;
    data[i + 1] = Math.random() * 255;
    data[i + 2] = Math.random() * 255;
    }
    }
    context.putImageData(imageData, 0, 0);
    image.src = canvas.toDataURL();
    };
    document.body.appendChild(gaussianNoiseButton);
    var scrambleImageButton = document.createElement('button');
    scrambleImageButton.innerHTML = 'Scramble Image';
    scrambleImageButton.onclick = function() {
    var image = document.querySelector('img');
    var canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    var context = canvas.getContext('2d');
    context.drawImage(image, 0, 0);
    var imageData = context.getImageData(0, 0, canvas.width, canvas.height);
    var data = imageData.data;
    var tileWidth = Math.floor(canvas.width / slider.value);
    var tileHeight = Math.floor(canvas.height / slider.value);
    var tiles = [];
    for (var y = 0; y < canvas.height; y += tileHeight) {
    for (var x = 0; x < canvas.width; x += tileWidth) {
    var tile = context.getImageData(x, y, tileWidth, tileHeight);
    tiles.push(tile);
    }
    }
    tiles.sort(function() {
    return Math.random() - 0.5;
    });
    var i = 0;
    for (var y = 0; y < canvas.height; y += tileHeight) {
    for (var x = 0; x < canvas.width; x += tileWidth) {
    context.putImageData(tiles[i], x, y);
    i++;
    }
    }
    image.src = canvas.toDataURL();
    };
    document.body.appendChild(scrambleImageButton);
    
    var clearButton = document.createElement('button');
    clearButton.innerHTML = 'Clear';
    clearButton.onclick = function() {
    var image = document.querySelector('img');
    image.src = '';
    };
    document.body.appendChild(clearButton);
    
    
    var NoiseCircularAreaButton = document.createElement('button');
    NoiseCircularAreaButton.innerHTML = 'Noise Circular Area';
    NoiseCircularAreaButton.onclick = function() {
    var image = document.getElementsByTagName('img')[0];
    var canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    var context = canvas.getContext('2d');
    context.drawImage(image, 0, 0);
    var imageData = context.getImageData(0, 0, canvas.width, canvas.height);
    var data = imageData.data;
    var x = 0;
    var y = 0;
    var radius = 0;
    var isDrawing = false;
    canvas.onmousedown = function(e) {
    isDrawing = true;
    x = e.offsetX;
    y = e.offsetY;
    radius = 0;
    };
    canvas.onmousemove = function(e) {
    if (isDrawing) {
    radius = Math.sqrt(Math.pow(e.offsetX - x, 2) + Math.pow(e.offsetY - y, 2));
    context.clearRect(0, 0, canvas.width, canvas.height);
    context.drawImage(image, 0, 0);
    context.beginPath();
    context.arc(x, y, radius, 0, 2 * Math.PI);
    context.strokeStyle = 'rgba(255, 0, 0, 0.5)';
    context.lineWidth = 3;
    context.stroke();
    }
    };
    canvas.onmouseup = function(e) {
    isDrawing = false;
    for (var i = 0; i < data.length; i += 4) {
    var dx = (i / 4) % canvas.width - x;
    var dy = Math.floor((i / 4) / canvas.width) - y;
    if (Math.sqrt(dx * dx + dy * dy) < radius) {
    if (Math.random() < slider.value / 100) {
      var j = Math.floor(Math.random() * data.length);
      data[i] = data[j];
      data[i + 1] = data[j + 1];
      data[i + 2] = data[j + 2];
    }
    }
    }
    context.putImageData(imageData, 0, 0);
    };
    document.body.appendChild(canvas);
    };
    document.body.appendChild(NoiseCircularAreaButton);
    
    document.body.style.overflow = 'hidden';
    
    clearButton.onclick = function() {
    location.reload();
    };
    
    var scrambleCircularAreaButton = document.createElement('button');
    scrambleCircularAreaButton.innerHTML = 'Scramble Circular Area';
    scrambleCircularAreaButton.onclick = function() {
    var image = document.getElementsByTagName('img')[0];
    var canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    var context = canvas.getContext('2d');
    context.drawImage(image, 0, 0);
    var imageData = context.getImageData(0, 0, canvas.width, canvas.height);
    var data = imageData.data;
    var x = 0;
    var y = 0;
    var radius = 0;
    var isDrawing = false;
    canvas.onmousedown = function(e) {
    isDrawing = true;
    x = e.offsetX;
    y = e.offsetY;
    radius = 0;
    };
    canvas.onmousemove = function(e) {
    if (isDrawing) {
    radius = Math.sqrt(Math.pow(e.offsetX - x, 2) + Math.pow(e.offsetY - y, 2));
    context.clearRect(0, 0, canvas.width, canvas.height);
    context.drawImage(image, 0, 0);
    context.beginPath();
    context.arc(x, y, radius, 0, 2 * Math.PI);
    context.stroke();
    }
    };
    canvas.onmouseup = function(e) {
    isDrawing = false;
    for (var i = 0; i < data.length; i += 4) {
    var dx = (i / 4) % canvas.width - x;
    var dy = Math.floor((i / 4) / canvas.width) - y;
    if (Math.sqrt(dx * dx + dy * dy) < radius) {
    if (Math.random() < slider.value / 100) {
      var j = Math.floor(Math.random() * data.length / 4) * 4;
      data[i] = data[j];
      data[i + 1] = data[j + 1];
      data[i + 2] = data[j + 2];
    }
    }
    }
    context.putImageData(imageData, 0, 0);
    };
    document.body.appendChild(canvas);
    };
    document.body.appendChild(scrambleCircularAreaButton);
    var buttons = document.querySelectorAll('button');
    for (var i = 0; i < buttons.length; i++) {
    buttons[i].style.fontSize = '20px';
    buttons[i].style.padding = '10px';
    buttons[i].style.margin = '10px';
    buttons[i].style.borderRadius = '10px';
    buttons[i].style.border = 'none';
    buttons[i].style.backgroundColor = '#eee';
    buttons[i].style.cursor = 'pointer';
    }
    
    var scrambleSliderText = document.createElement('div');
    scrambleSliderText.innerHTML = 'Scramble ratio';
    document.body.appendChild(scrambleSliderText);
    
    scrambleSliderText.style.position = 'absolute';
    scrambleSliderText.style.top = '50px';
    scrambleSliderText.style.left = '180px';
    
    scrambleSliderText.style.fontSize = '15px';
    scrambleSliderText.style.fontFamily = 'sans-serif';
    scrambleSliderText.style.color = '#666';
    
    
    
    
    var twitterLink = document.createElement('a');
    twitterLink.innerHTML = '@altunenes';
    twitterLink.href = 'https://github.com/altunenes';
    twitterLink.style.position = 'fixed';
    twitterLink.style.bottom = '10px';
    twitterLink.style.right = '10px';
    twitterLink.style.fontSize = '15px';
    twitterLink.style.fontFamily = 'sans-serif';
    twitterLink.style.color = '#666';
    document.body.appendChild(twitterLink);
    document.body.style.overflow = 'auto';
    
    
    var about = document.createElement('div');
    about.innerHTML = 'About';
    about.style.position = 'absolute';
    about.style.top = '0px';
    about.style.right = '0px';
    about.style.backgroundColor = '#ccc';
    about.style.padding = '10px';
    about.style.cursor = 'pointer';
    document.body.appendChild(about);
    var aboutText = document.createElement('div');
    aboutText.innerHTML = '<h1>About</h1><p>"Scramble selected areas" is still under construction</p><p>Made by <a href="https://altunenes.github.io/">enes altun, 2022</a></p>';
    aboutText.style.position = 'absolute';
    aboutText.style.top = '0px';
    aboutText.style.right = '0px';
    aboutText.style.backgroundColor = '#ccc';
    aboutText.style.padding = '10px';
    aboutText.style.cursor = 'pointer';
    aboutText.style.display = 'none';
    document.body.appendChild(aboutText);
    about.addEventListener('click', function() {
      aboutText.style.display = 'block';
    });
    aboutText.addEventListener('click', function() {
      aboutText.style.display = 'none';
    });

/* cfa test
var scrambleImageButton = document.createElement('button');
	    scrambleImageButton.innerHTML = 'Scramble Image';
	    scrambleImageButton.onclick = function() {
	    var image = document.getElementsByTagName('img')[0];
	    var canvas = document.createElement('canvas');
	    canvas.width = image.width;
	    canvas.height = image.height;
	    var context = canvas.getContext('2d');
	    context.drawImage(image, 0, 0);
	    var imageData = context.getImageData(0, 0, canvas.width, canvas.height);
	    var data = imageData.data;
	    var frameWidth = Math.round(canvas.width / slider.value);
	    var frameHeight = Math.round(canvas.height / slider.value);
	    var frameData = [];
	    for (var y = 0; y < canvas.height; y += frameHeight) {
	    for (var x = 0; x < canvas.width; x += frameWidth) {
	    var sum = 0;
	    for (var i = 0; i < frameWidth; i++) {
	    for (var j = 0; j < frameHeight; j++) {
	    var index = (x + i + (y + j) * canvas.width) * 4;
	    sum += data[index] + data[index + 1] + data[index + 2];
	    }
	    }
	    frameData.push(sum / (frameWidth * frameHeight * 3));
	    }
	    }
	    frameData.sort(function(a, b) {
	    return a - b;
	    });
	    for (var y = 0; y < canvas.height; y += frameHeight) {
	    for (var x = 0; x < canvas.width; x += frameWidth) {
	    var sum = 0;
	    for (var i = 0; i < frameWidth; i++) {
	    for (var j = 0; j < frameHeight; j++) {
	    var index = (x + i + (y + j) * canvas.width) * 3;
	    sum += data[index] + data[index + 1] + data[index + 2];
	    }
	    }
	    var average = sum / (frameWidth * frameHeight * 4);
	    var frameIndex = frameData.indexOf(average);
	    for (var i = 0; i < frameWidth; i++) {
	    for (var j = 0; j < frameHeight; j++) {
	    var index = (x + i + (y + j) * canvas.width) * 4;
	    var index2 = (frameIndex % slider.value * frameWidth + i + (Math.floor(frameIndex / slider.value) * frameHeight + j) * canvas.width) * 12;
	    data[index] = data[index2];
	    data[index + 1] = data[index2 + 1];
	    data[index + 2] = data[index2 + 2];
	    }
	    }
	    }
	    }
	    context.putImageData(imageData, 0, 0);
	    var image2 = document.createElement('img');
	    image2.src = canvas.toDataURL();
	    document.body.appendChild(image2);
	    };
	    document.body.appendChild(scrambleImageButton); /*/


var scrambleImage = document.createElement('button');
scrambleImage.innerHTML = 'Scramble Image';
scrambleImage.onclick = function() {
  var img = document.getElementsByTagName('img')[0];
  var canvas = document.createElement('canvas');
  canvas.width = img.width;
  canvas.height = img.height;
  var ctx = canvas.getContext('2d');
  ctx.drawImage(img, 0, 0);
  var imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
  var data = imageData.data;
  var width = imageData.width;
  var height = imageData.height;
  var buttonWidth = 20;
  var buttonHeight = 20;
  var buttonWidthCount = Math.floor(width / buttonWidth);
  var buttonHeightCount = Math.floor(height / buttonHeight);
  var buttonCount = buttonWidthCount * buttonHeightCount;
  var buttonColors = [];
  for (var i = 0; i < buttonCount; i++) {
    var x = i % buttonWidthCount;
    var y = Math.floor(i / buttonWidthCount);
    var r = 0;
    var g = 0;
    var b = 0;
    for (var j = 0; j < buttonWidth; j++) {
      for (var k = 0; k < buttonHeight; k++) {
        var index = (y * buttonHeight + k) * width * 4 + (x * buttonWidth + j) * 4;
        r += data[index];
        g += data[index + 1];
        b += data[index + 2];
      }
    }
    r = Math.floor(r / (buttonWidth * buttonHeight));
    g = Math.floor(g / (buttonWidth * buttonHeight));
    b = Math.floor(b / (buttonWidth * buttonHeight));
    buttonColors.push([r, g, b]);
  }
  var buttonPositions = [];
  for (var i = 0; i < buttonCount; i++) {
    buttonPositions.push(i);
  }
  var scrambleRatio = 0.5;
  for (var i = 0; i < buttonCount; i++) {
    var j = Math.floor(Math.random() * buttonCount);
    var temp = buttonPositions[i];
    buttonPositions[i] = buttonPositions[j];
    buttonPositions[j] = temp;
  }
  var scrambleCount = Math.floor(buttonCount * scrambleRatio);
  for (var i = 0; i < scrambleCount; i++) {
    var j = Math.floor(Math.random() * buttonCount);
    var temp = buttonPositions[i];
    buttonPositions[i] = buttonPositions[j];
    buttonPositions[j] = temp;
  }
  var buttons = [];
  for (var i = 0; i < buttonCount; i++) {
    var button = document.createElement('button');
    button.style.width = buttonWidth + 'px';
    button.style.height = buttonHeight + 'px';
    button.style.backgroundColor = 'rgb(' + buttonColors[i][0] + ',' + buttonColors[i][1] + ',' + buttonColors[i][2] + ')';
    button.style.position = 'absolute';
    button.style.left = (buttonPositions[i] % buttonWidthCount * buttonWidth) + 'px';
    button.style.top = (Math.floor(buttonPositions[i] / buttonWidthCount) * buttonHeight) + 'px';
    buttons.push(button);
    document.body.appendChild(button);
  }
  var buttonIndex = 0;
  var interval = setInterval(function() {
    if (buttonIndex >= buttonCount) {
      clearInterval(interval);
      return;
    }
    var button = buttons[buttonIndex];
    button.style.left = (buttonIndex % buttonWidthCount * buttonWidth) + 'px';
    button.style.top = (Math.floor(buttonIndex / buttonWidthCount) * buttonHeight) + 'px';
    buttonIndex++;
  }, 10);
};
document.body.appendChild(scrambleImage);