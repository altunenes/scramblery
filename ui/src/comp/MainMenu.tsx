import { Link } from "react-router-dom";
import { Github, Bug } from 'lucide-react';

const MainMenu = () => {
  const handleExternalLink = async (url: string) => {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(url);
    } catch (error) {
      console.error('Failed to open external link:', error);
    }
  };

  return (
    <div className="menu-container">
      <h1>Scramblery</h1>
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

      <div style={{ marginTop: '3rem' }}>
        <div style={{ display: 'flex', justifyContent: 'center', gap: '3rem', marginBottom: '2rem' }}>
          <button 
            style={{ 
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: '#4b5563'
            }}
            onClick={() => handleExternalLink('https://github.com/altunenes/scramblery')}
            title="Source Code"
          >
            <Github size={20} />
          </button>
          <button 
            style={{ 
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: '#4b5563'
            }}
            onClick={() => handleExternalLink('https://github.com/altunenes/scramblery/issues')}
            title="Report Issue"
          >
            <Bug size={20} />
          </button>
        </div>

        <div style={{ 
          fontStyle: 'italic',
          fontSize: '0.75rem',
          color: '#4b5563',
          marginTop: '1rem'
        }}>
          If you use this tool in your research, please cite:{' '}
          <button 
            style={{
              background: 'none',
              border: 'none',
              color: '#2563eb',
              cursor: 'pointer',
              padding: 0,
              fontStyle: 'normal'
            }}
            onClick={() => handleExternalLink('https://doi.org/10.5281/zenodo.7484576')}
          >
            DOI: 10.5281/zenodo.7484576
          </button>
        </div>
      </div>
    </div>
  );
};

export default MainMenu;