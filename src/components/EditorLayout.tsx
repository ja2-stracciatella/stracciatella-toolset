import { ReactNode, useMemo } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { Layout, Menu, Typography } from "antd";

import { useMods } from "../state/mods";
import "./EditorLayout.css";
import { ROUTES } from "../EditorRoutes";

const { Header, Sider } = Layout;
const { Item } = Menu;

interface LayoutProps {
  children: ReactNode;
}

function SideMenu() {
  const location = useLocation();
  const navigate = useNavigate();
  const selectedKeys = useMemo(() => {
    const selectedRoute = ROUTES.find((v) => v.url === location.pathname);
    return selectedRoute ? [selectedRoute.id] : [];
  }, [location.pathname]);
  const menu = useMemo(() => {
    return (
      <Menu theme="dark" selectedKeys={selectedKeys}>
        {ROUTES.map((r) => (
          <Item key={r.id} onClick={() => navigate(r.url)}>
            {r.label}
          </Item>
        ))}
      </Menu>
    );
  }, [navigate, selectedKeys]);

  return menu;
}

export function EditorLayout({ children }: LayoutProps) {
  const { selectedMod } = useMods();
  const modSuffix = useMemo(
    () => (selectedMod ? ` - ${selectedMod.name}` : ""),
    [selectedMod]
  );
  return (
    <Layout className="editor-layout">
      <Header className="editor-layout-header">
        <Typography.Title level={4} type="secondary" className="editor-layout-title">
          Stracciatella Toolset{modSuffix}
        </Typography.Title>
      </Header>
      <Layout>
        <Sider width="300px" className="editor-layout-sidebar">
          <SideMenu />
        </Sider>
        <Layout>
          <div className="editor-layout-content">{children}</div>
        </Layout>
      </Layout>
    </Layout>
  );
}
