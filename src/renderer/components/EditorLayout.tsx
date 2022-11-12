import { ReactNode, useMemo } from 'react';
import { NavigateFunction, useLocation, useNavigate } from 'react-router-dom';
import { Layout, Menu, Typography } from 'antd';

import { useMods } from '../state/mods';
import './EditorLayout.css';
import { MENU, MenuItem } from '../EditorRoutes';
import { ItemType } from 'antd/lib/menu/hooks/useItems';

const { Header, Sider } = Layout;

interface LayoutProps {
  children: ReactNode;
}

function routeToItem(
  navigate: NavigateFunction,
  parentPath: string,
  menuItem: MenuItem
): ItemType {
  const path = `${parentPath}/${menuItem.id}`;
  if (menuItem.type === 'Item') {
    return {
      key: path,
      label: menuItem.label,
      onClick: () => navigate(path),
    };
  }
  const sorted = [...menuItem.children];
  sorted.sort((a, b) =>
    a.label.localeCompare(b.label, 'en', { ignorePunctuation: true })
  );
  return {
    key: path,
    label: menuItem.label,
    children: sorted.map((c) => routeToItem(navigate, path, c)),
  };
}

function SideMenu() {
  const location = useLocation();
  const navigate = useNavigate();
  const selectedKeys = useMemo(() => {
    return [location.pathname];
  }, [location.pathname]);
  const defaultOpenKeys = useMemo(() => {
    const split = location.pathname.split('/');

    if (split.length > 2) {
      return split.reduce((prev, curr, index): string[] => {
        if (index > 0) {
          const parent = prev[0] ?? '';
          return [`${parent}/${curr}`, ...prev];
        }
        return [];
      }, [] as string[]);
    }
    return [];
  }, [location.pathname]);
  const items = useMemo(() => {
    const [dashboard, ...sorted] = MENU;
    sorted.sort((a, b) =>
      a.label.localeCompare(b.label, 'en', { ignorePunctuation: true })
    );
    return [dashboard, ...sorted].map((r) => routeToItem(navigate, '', r));
  }, [navigate]);

  return (
    <Menu
      mode="inline"
      theme="dark"
      items={items}
      selectedKeys={selectedKeys}
      defaultOpenKeys={defaultOpenKeys}
    />
  );
}

export function EditorLayout({ children }: LayoutProps) {
  const { selectedMod } = useMods();
  const modSuffix = useMemo(
    () => (selectedMod ? ` - ${selectedMod.name}` : ''),
    [selectedMod]
  );
  return (
    <Layout className="editor-layout">
      <Header className="editor-layout-header">
        <Typography.Title
          level={4}
          type="secondary"
          className="editor-layout-title"
        >
          Stracciatella Toolset{modSuffix}
        </Typography.Title>
      </Header>
      <Layout>
        <Sider width="300px" className="editor-layout-sidebar">
          <SideMenu />
        </Sider>
        <Layout>{children}</Layout>
      </Layout>
    </Layout>
  );
}
