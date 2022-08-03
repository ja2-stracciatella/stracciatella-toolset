import { BrowserRouter, Route, Routes } from "react-router-dom";
import "antd/dist/antd.compact.less";

import { ModsProvider } from "./state/mods";

import { useMemo } from "react";
import { ROUTES } from "./EditorRoutes";
import "./App.css";
import { ToolsetConfigProvider } from "./state/toolset";
import { WithToolsetConfig } from "./components/WithToolsetConfig";
import { WithOpenMod } from "./components/WithOpenMod";
import { EditorLayout } from "./components/EditorLayout";
import { FilesProvider } from "./state/files";

function App() {
  const routes = useMemo(() => {
    return ROUTES.map((r) => (
      <Route key={r.id} path={r.url} element={<r.component />} />
    ));
  }, []);
  return (
    <div className="app-root">
      <ModsProvider>
        <ToolsetConfigProvider>
          <FilesProvider>
            <WithToolsetConfig>
              <WithOpenMod>
                <BrowserRouter>
                  <EditorLayout>
                    <Routes>{routes}</Routes>
                  </EditorLayout>
                </BrowserRouter>
              </WithOpenMod>
            </WithToolsetConfig>
          </FilesProvider>
        </ToolsetConfigProvider>
      </ModsProvider>
    </div>
  );
}

export default App;
