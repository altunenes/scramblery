var importImageButton = document.createElement('button');
importImageButton.innerHTML = 'Import Image';
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
slider.min = 0;
slider.max = 150;
slider.value = 0;
slider.oninput = function() {
  console.log(slider.value);
};
document.body.appendChild(slider);

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
