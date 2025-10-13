import { Layout } from 'antd';
import './FullSizeDialogLayout.css';

export function FullSizeDialogLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <Layout className="full-size-dialog-layout-root">
      <div className="full-size-dialog-layout-content">{children}</div>
    </Layout>
  );
}
