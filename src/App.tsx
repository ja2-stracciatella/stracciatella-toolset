import { BrowserRouter, Route, Routes } from "react-router-dom";
import 'bootstrap/dist/css/bootstrap.min.css';

import { ModsProvider } from "./state/mods";

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
