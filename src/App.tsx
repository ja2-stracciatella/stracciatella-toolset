import React from "react";
import "./App.css";
import "bootstrap/dist/css/bootstrap.min.css";
import { BrowserRouter, Route, Routes } from "react-router-dom";
import { Initialization } from "./pages/Initialization";
import { ModsProvider } from "./state/mods";
import { Open } from "./pages/Open";

function App() {
  return (
    <div className="app-root">
      <ModsProvider>
        <BrowserRouter>
          <Routes>
            <Route path="" element={<Initialization />} />
            <Route path="open" element={<Open />} />
          </Routes>
        </BrowserRouter>
      </ModsProvider>
    </div>
  );
}

export default App;
