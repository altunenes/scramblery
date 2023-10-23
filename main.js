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