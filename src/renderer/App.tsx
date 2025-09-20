import { BrowserRouter, Route, Routes } from 'react-router-dom';
import 'antd/dist/reset.css';
import { useMemo } from 'react';
import { ROUTES } from './EditorRoutes';
import './App.css';
import { WithToolsetConfig } from './components/WithToolsetConfig';
import { WithOpenMod } from './components/WithOpenMod';
import { EditorLayout } from './components/EditorLayout';
import { Provider } from 'react-redux';
import { appStore } from './state/store';

export function AppWithoutProviders() {
  const routes = useMemo(() => {
    return ROUTES.map((r) => (
      <Route key={r.id} path={r.url} element={<r.component />} />
    ));
  }, []);

  return (
    <WithToolsetConfig>
      <WithOpenMod>
        <BrowserRouter>
          <EditorLayout>
            <Routes>{routes}</Routes>
          </EditorLayout>
        </BrowserRouter>
      </WithOpenMod>
    </WithToolsetConfig>
  );
}

export default function App() {
  return (
    <div className="app-root">
      <Provider store={appStore}>
        <AppWithoutProviders />
      </Provider>
    </div>
  );
}
