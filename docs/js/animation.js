var importImage = document.createElement('input');
importImage.type = 'file';
importImage.style.position = 'absolute';
importImage.style.top = '0px';
importImage.style.right = '10px';
importImage.accept = 'image/*';

importImage.onchange = function(e) {
  var file = e.target.files[0];
  var reader = new FileReader();
  reader.onload = function(e) {
    var img = document.createElement('img');
    img.src = e.target.result;
    document.body.appendChild(img);
  };
  reader.readAsDataURL(file);
};
document.body.appendChild(importImage);

var scrambleImage = document.createElement('button');
scrambleImage.innerHTML = 'Magic';
scrambleImage.style.position = 'absolute';
scrambleImage.style.top = '10%';
scrambleImage.style.right = '20px';
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
  var scrambleRatio = 0.01;
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


var importImage = document.getElementsByTagName('input')[0];
importImage.onchange = function(e) {
  var file = e.target.files[0];
  var reader = new FileReader();
  reader.onload = function(e) {
    var img = document.createElement('img');
    img.src = e.target.result;
    document.body.appendChild(img);
    img.style.display = 'none';
  };
  reader.readAsDataURL(file);
};

scrambleImage.style.position = 'absolute';
scrambleImage.style.top = '10%';
scrambleImage.style.right = '10%';
scrambleImage.style.width = '100px';
scrambleImage.style.height = '100px';
scrambleImage.style.backgroundColor = '#00ff00';
scrambleImage.style.border = 'none';
scrambleImage.style.borderRadius = '50%';
scrambleImage.style.boxShadow = '0px 0px 10px #000';
scrambleImage.style.cursor = 'pointer';
scrambleImage.style.outline = 'none';
scrambleImage.style.opacity = '0.5';
scrambleImage.style.zIndex = '9999';
var scrambleImage = document.createElement('div');
scrambleImage.innerHTML = 'Add Image';
scrambleImage.style.position = 'absolute';
scrambleImage.style.top = '0px';
scrambleImage.style.right = '20px';
scrambleImage.style.width = '100px';
scrambleImage.style.height = '100px';
scrambleImage.style.lineHeight = '100px';
scrambleImage.style.textAlign = 'center';
scrambleImage.style.fontSize = '20px';
scrambleImage.style.color = '#fff';
scrambleImage.style.zIndex = '9998';
document.body.appendChild(scrambleImage);


var github = document.createElement('a');
github.href = 'https://github.com/altunenes/scramblery';
github.target = '_blank';
github.style.position = 'absolute';
github.style.right = '4%';
github.style.bottom = '15%';
github.style.transform = 'translate(50%, 50%)';
github.style.width = '80px';
github.style.height = '80px';
github.style.backgroundImage = 'url(https://github.githubassets.com/images/modules/logos_page/GitHub-Mark.png)';
github.style.backgroundSize = 'contain';
github.style.backgroundRepeat = 'no-repeat';
github.style.backgroundPosition = 'center';
document.body.appendChild(github);

github.style.borderRadius = '70%';
github.style.boxShadow = '0 0 20px rgba(0, 0, 0, 0.5)';

var aboutButton = document.createElement('div');
aboutButton.innerHTML = 'About';
aboutButton.style.position = 'fixed';
aboutButton.style.top = '50%';
aboutButton.style.right = '0px';
aboutButton.style.backgroundColor = '#00FF00';
aboutButton.style.padding = '20px';
aboutButton.style.cursor = 'pointer';
aboutButton.style.fontFamily = 'sans-serif';
aboutButton.style.fontSize = '20px';
aboutButton.style.fontWeight = 'bold';
aboutButton.style.color = '#000000';
aboutButton.style.textAlign = 'center';
aboutButton.style.border = '1px solid #000000';
aboutButton.style.borderRadius = '10px';
aboutButton.style.boxShadow = '0px 0px 10px #000000';
aboutButton.style.zIndex = '9999';
document.body.appendChild(aboutButton);
var aboutText = document.createElement('div');
aboutText.innerHTML = '<p>Author: <a href="http://www.github.com/altunenes">Enes Altun</a></p><p>Usage: import image then clicks on the magic button.></p>';
aboutText.style.position = 'fixed';
aboutText.style.top = '50%';
aboutText.style.right = '0px';
aboutText.style.backgroundColor = '#00FF00';
aboutText.style.padding = '10px';
aboutText.style.cursor = 'pointer';
aboutText.style.fontFamily = 'sans-serif';
aboutText.style.fontSize = '20px';
aboutText.style.fontWeight = 'bold';
aboutText.style.color = '#000000';
aboutText.style.textAlign = 'center';
aboutText.style.border = '1px solid #000000';
aboutText.style.borderRadius = '10px';
aboutText.style.boxShadow = '0px 0px 10px #000000';
aboutText.style.zIndex = '9999';
aboutText.style.display = 'none';
document.body.appendChild(aboutText);
aboutButton.addEventListener('click', function() {
  if (aboutText.style.display === 'none') {
    aboutText.style.display = 'block';
  } else {
    aboutText.style.display = 'none';
  }
});