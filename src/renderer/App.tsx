import { BrowserRouter, Route, Routes } from 'react-router-dom';
import 'antd/dist/reset.css';
import '@ant-design/v5-patch-for-react-19';
import { useEffect, useMemo } from 'react';
import { ROUTES } from './EditorRoutes';
import './App.css';
import { WithToolsetConfig } from './components/WithToolsetConfig';
import { WithSelectedMod } from './components/selectedMod/WithSelectedMod';
import { EditorLayout } from './components/EditorLayout';
import { Provider } from 'react-redux';
import { appStore } from './state/store';
import { setCloseRequested } from './state/toolset';
import { useAppDispatch } from './hooks/state';
import { useSelectedMod } from './hooks/useSelectedMod';
import { invokeWithSchema } from './lib/invoke';
import z from 'zod';

function MainActionsHandler() {
  const dispatch = useAppDispatch();
  const { data: selectedMod } = useSelectedMod();

  useEffect(() => {
    window.electronAPI.onMainAction((payload) => {
      console.log('Toolset close confirm', payload);
      if (payload.type === 'toolset_close_requested') {
        if (!selectedMod) {
          invokeWithSchema(z.unknown(), 'toolset_close_confirm');
        } else {
          dispatch(setCloseRequested(true));
        }
      }
    });
  }, [dispatch, selectedMod]);

  return null;
}

export function AppWithoutProviders() {
  const routes = useMemo(() => {
    return ROUTES.map((r) => (
      <Route key={r.id} path={r.url} element={<r.component />} />
    ));
  }, []);

  return (
    <>
      <MainActionsHandler />
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
