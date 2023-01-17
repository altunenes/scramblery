let img;
let squareSize = 8;
let squareSpacing = 2;
let noiseScale = 0.001;
let noiseDimensions = 44;
let input;

function setup() {
    createCanvas(800, 600);
    noStroke();
    input = createFileInput(handleFile);
    input.hide();

    let importImageButton = select('#import-image-button');
    importImageButton.mousePressed(() => {
        input.elt.click();
    });

    let aboutButton = document.getElementById("about-button");
    let aboutSection = document.getElementById("about-section");
    aboutButton.onclick = function(){
        if(aboutSection.style.display === "none"){
            aboutSection.style.display = "block";
        } else {
            aboutSection.style.display = "none";
        }
    }
    let fullscreenButton = document.getElementById("fullscreen-button");

fullscreenButton.addEventListener("click", function(){
    let canvas = document.querySelector("canvas");
    canvas.requestFullscreen();
});
}

function handleFile(file) {
    if (file.type === 'image') {
        img = loadImage(file.data, () => {
            img.loadPixels();
        });
    }
}

function draw() {
    background(0);
    if (img) {
        for (let x = 0; x < width; x += squareSize + squareSpacing) {
            for (let y = 0; y < height; y += squareSize + squareSpacing) {
                let noiseValX = noise(x * noiseScale, y * noiseScale, frameCount * noiseScale, noiseDimensions);
                let noiseValY = noise((x + 100) * noiseScale, (y + 100) * noiseScale, frameCount * noiseScale, noiseDimensions);
                let angle = noiseValX * TWO_PI;
                let scale = noiseValY;
                let imgX = int(noiseValX * img.width);
                let imgY = int(noiseValY * img.height);
                let pixelIndex = (imgX + imgY * img.width) * 4;
                fill(img.pixels[pixelIndex], img.pixels[pixelIndex + 1], img.pixels[pixelIndex + 2], img.pixels[pixelIndex + 3]);
                rect(x, y, squareSize, squareSize);
                //*
                //*/rect(x, y, squareSize, squareSize);
                //*/stroke(255, 255, 255, 100); // semi-transparent, you can adjust the alpha value
                //*/strokeWeight(1); // adjust the weight of the stroke, increasing the value will make the line thicker
                //*/rect(x, y, squareSize, squareSize);
                //*/
            }
        }
    }
}