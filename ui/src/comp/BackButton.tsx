import { Link } from 'react-router-dom';
import { ChevronLeft } from 'lucide-react';

const BackButton = () => {
  return (
    <Link 
      to="/" 
      className="fixed top-4 left-4 inline-flex items-center px-3 py-2 bg-white hover:bg-gray-50 
        text-gray-700 text-sm font-medium rounded-lg border border-gray-200 
        shadow-sm transition-colors duration-200 gap-1"
    >
      <ChevronLeft size={18} />
      Back to Menu
    </Link>
  );
};

export default BackButton;