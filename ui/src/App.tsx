import { BrowserRouter as Router, Routes, Route, Link } from "react-router-dom";
import "./App.css";
import SingleImage from "./singleimage";
import FolderProcess from "./folderprocess"; 


function VideoProcess() {
  return (
    <div className="page-container">
      <h2>Video Scrambling</h2>
      <p>Upload and scramble video content.</p>
      {/* TODO */}
    </div>
  );
}

function MainMenu() {
  return (
    <div className="menu-container">
      <h1>Image Scrambling Tool</h1>
      <p>Select processing type:</p>
      <div className="button-container">
        <Link to="/single" className="menu-button">
          Single Image Processing
        </Link>
        <Link to="/folder" className="menu-button">
          Multiple Image Processing
        </Link>
        <Link to="/video" className="menu-button">
          Video Processing
        </Link>
      </div>
    </div>
  );
}

function App() {
  return (
    <Router>
      <div className="container">
        <Routes>
          <Route path="/" element={<MainMenu />} />
          <Route path="/single" element={<SingleImage />} />
          <Route path="/folder" element={<FolderProcess />} />
          <Route path="/video" element={<VideoProcess />} />
        </Routes>
      </div>
    </Router>
  );
}

export default App;