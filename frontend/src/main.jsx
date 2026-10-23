import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { AuthProvider } from './lib/auth'
import { LanguageProvider } from './lib/language'
import App from './App'
import './styles.css'

ReactDOM.createRoot(document.getElementById('root')).render(
  <React.StrictMode><BrowserRouter><LanguageProvider><AuthProvider><App /></AuthProvider></LanguageProvider></BrowserRouter></React.StrictMode>
)

