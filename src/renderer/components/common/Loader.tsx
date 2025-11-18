import { LoadingOutlined } from '@ant-design/icons';
import { Spin } from 'antd';

interface LoaderProps {
  size?: 'small' | 'default' | 'large';
}

const ICON = <LoadingOutlined />;

export function Loader({ size = 'small' }: LoaderProps) {
  return <Spin size={size} indicator={ICON} />;
}
