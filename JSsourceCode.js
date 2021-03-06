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
var scrambleRectangleAreaButton = document.createElement('button');
scrambleRectangleAreaButton.innerHTML = 'Scramble Rectangle Area';
scrambleRectangleAreaButton.onclick = function() {
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
  var width = 0;
  var height = 0;
  var isDrawing = false;
  canvas.onmousedown = function(e) {
    isDrawing = true;
    x = e.offsetX;
    y = e.offsetY;
    width = 0;
    height = 0;
  };
  canvas.onmousemove = function(e) {
    if (isDrawing) {
      width = e.offsetX - x;
      height = e.offsetY - y;
      context.clearRect(0, 0, canvas.width, canvas.height);
      context.drawImage(image, 0, 0);
      context.beginPath();
      context.rect(x, y, width, height);
      context.stroke();
    }
  };
  canvas.onmouseup = function(e) {
    isDrawing = false;
    for (var i = 0; i < data.length; i += 4) {
      var dx = (i / 4) % canvas.width - x;
      var dy = Math.floor((i / 4) / canvas.width) - y;
      if (dx >= 0 && dx < width && dy >= 0 && dy < height) {
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
document.body.appendChild(scrambleRectangleAreaButton);

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
