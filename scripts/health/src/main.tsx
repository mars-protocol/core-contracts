import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import './index.css'
import {SWRConfig} from "swr";

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
      <SWRConfig
          value={{
              revalidateOnFocus: false,
              revalidateOnReconnect: false,
              revalidateIfStale: false,
              keepPreviousData: false,
          }}
      >
    <App />
      </SWRConfig>
  </React.StrictMode>,
)
