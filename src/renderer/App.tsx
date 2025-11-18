import { BrowserRouter, Route, Routes } from 'react-router-dom';
import 'antd/dist/reset.css';
import '@ant-design/v5-patch-for-react-19';
import { useMemo } from 'react';
import { ROUTES } from './EditorRoutes';
import './App.css';
import { WithToolsetConfig } from './components/WithToolsetConfig';
import { WithSelectedMod } from './components/selectedMod/WithSelectedMod';
import { EditorLayout } from './components/layout/EditorLayout';
import { Provider } from 'react-redux';
import { appStore } from './state/store';
import { ListenAll } from './components/events/ListenAll';

export function AppWithoutProviders() {
  const routes = useMemo(() => {
    return ROUTES.map((r) => (
      <Route key={r.id} path={r.url} element={<r.component />} />
    ));
  }, []);

  return (
    <>
      <ListenAll />
      <WithToolsetConfig>
        <WithSelectedMod>
          <BrowserRouter>
            <EditorLayout>
              <Routes>{routes}</Routes>
            </EditorLayout>
          </BrowserRouter>
        </WithSelectedMod>
      </WithToolsetConfig>
    </>
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
