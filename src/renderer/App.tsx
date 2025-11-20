import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import 'antd/dist/reset.css';
import '@ant-design/v5-patch-for-react-19';
import { useMemo } from 'react';
import { ROUTES } from './EditorRoutes';
import './App.css';
import { WithToolsetConfig } from './components/WithToolsetConfig';
import { WithSelectedMod } from './components/selectedMod/WithSelectedMod';
import { ToolsetLayout } from './components/layout/ToolsetLayout';
import { Provider } from 'react-redux';
import { createAppStore } from './state/store';
import { ListenAll } from './components/events/ListenAll';
import { ConfigProvider, ThemeConfig } from 'antd';

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
            <ToolsetLayout>
              <Routes>
                {routes}
                <Route
                  path="*"
                  element={<Navigate to="/dashboard" replace />}
                />
              </Routes>
            </ToolsetLayout>
          </BrowserRouter>
        </WithSelectedMod>
      </WithToolsetConfig>
    </>
  );
}

const appStore = createAppStore();
const THEME_CONFIG: ThemeConfig = {
  token: {
    colorPrimary: '#9d1e1c',
  },
};

export default function App() {
  return (
    <div className="app-root">
      <ConfigProvider theme={THEME_CONFIG}>
        <Provider store={appStore}>
          <AppWithoutProviders />
        </Provider>
      </ConfigProvider>
    </div>
  );
}
