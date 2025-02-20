import { BrowserRouter as Router, Routes, Route } from "react-router-dom";
import "./App.css";
import MainMenu from "./comp/MainMenu";
import SingleImage from "./singleimage";
import FolderProcess from "./folderprocess"; 
import VideoProcess from "./videoprocess";

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