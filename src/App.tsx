import { BrowserRouter, Route, Routes } from "react-router-dom";
import "antd/dist/antd.compact.css";

import { ModsProvider } from "./state/mods";

import "./App.css";
import { useMemo } from "react";
import { ROUTES } from "./EditorRoutes";

function App() {
  const routes = useMemo(() => {
    return ROUTES.map((r) => (
      <Route key={r.id} path={r.url} element={<r.component />} />
    ));
  }, []);
  return (
    <div className="app-root">
      <ModsProvider>
        <BrowserRouter>
          <Routes>{routes}</Routes>
        </BrowserRouter>
      </ModsProvider>
    </div>
  );
}

export default App;
