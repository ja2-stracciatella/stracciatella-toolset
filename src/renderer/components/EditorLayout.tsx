import { ReactNode, useMemo } from 'react';
import { NavigateFunction, useLocation, useNavigate } from 'react-router-dom';
import { Badge, Layout, Menu, MenuProps, Space, Typography } from 'antd';

import './EditorLayout.css';
import { Item, MENU, MenuItem } from '../EditorRoutes';
import { useAppSelector } from '../hooks/state';
import { useFileModified } from '../hooks/files';

type ItemType = NonNullable<MenuProps['items']>[number];

const { Header, Sider } = Layout;

interface LayoutProps {
  children: ReactNode;
}

function MenuItemLabel({ item }: { item: Item }) {
  const modified = useFileModified(item.file ?? '');
  if (modified) {
    return (
      <Space size="small">
        {item.label}
        <Badge dot color="rgba(255,255,255,0.65)" />
      </Space>
    );
  }

  return item.label;
}

function routeToItem(
  navigate: NavigateFunction,
  parentPath: string,
  menuItem: MenuItem,
): ItemType {
  const path = `${parentPath}/${menuItem.id}`;
  if (menuItem.type === 'Item') {
    return {
      key: path,
      label: <MenuItemLabel item={menuItem} />,
      onClick: () => navigate(path),
    };
  }
  const sorted = [...menuItem.children];
  sorted.sort((a, b) =>
    a.label.localeCompare(b.label, 'en', { ignorePunctuation: true }),
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
      a.label.localeCompare(b.label, 'en', { ignorePunctuation: true }),
    );
    return [...(dashboard ? [dashboard] : []), ...sorted].map((r) =>
      routeToItem(navigate, '', r),
    );
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
  const selectedModName = useAppSelector((s) => s.mods.selected.data?.name);
  const modSuffix = useMemo(
    () => (selectedModName ? ` - ${selectedModName}` : ''),
    [selectedModName],
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
