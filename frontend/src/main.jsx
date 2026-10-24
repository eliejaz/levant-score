import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { AuthProvider } from './lib/auth'
import { LanguageProvider } from './lib/language'
import App from './App'
import { warmApi } from './lib/api'
import { isLocalMode } from './lib/runtime'
import './styles.css'

if(isLocalMode) navigator.serviceWorker?.register('/sw.js').catch(()=>{})
else warmApi()

ReactDOM.createRoot(document.getElementById('root')).render(
  <React.StrictMode><BrowserRouter><LanguageProvider><AuthProvider><App /></AuthProvider></LanguageProvider></BrowserRouter></React.StrictMode>
)

