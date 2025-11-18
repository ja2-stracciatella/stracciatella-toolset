import { Flex, theme } from 'antd';
import { useMemo } from 'react';

const FLEX_BASE_STYLE = {
  height: '100vh',
  width: '100vw',
  overflowX: 'hidden',
  overflowY: 'hidden',
} as const;

const DIALOG_STYLE = {
  position: 'relative',
  background: 'white',
  width: 'calc(100% - 20px)',
  maxWidth: '1024px',
  maxHeight: 'calc(100% - 20px)',
  minHeight: '60vh',
  margin: '10px',
  padding: '20px',
  overflow: 'auto',
  flexGrow: 1,
} as const;

export function FullSizeDialogLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const {
    token: { colorBgLayout },
  } = theme.useToken();
  const FLEX_STYLE = useMemo(
    () => ({
      ...FLEX_BASE_STYLE,
      backgroundColor: colorBgLayout,
    }),
    [colorBgLayout],
  );

  return (
    <Flex vertical align="center" justify="center" style={FLEX_STYLE}>
      <div style={DIALOG_STYLE}>{children}</div>
    </Flex>
  );
}
