import { Spin } from 'antd';
import { LoadingOutlined } from '@ant-design/icons';

import './FullSizeLoader.css';

const antIcon = <LoadingOutlined style={{ fontSize: 24 }} spin />;

export function FullSizeLoader() {
  return (
    <div className="full-size-loader">
      <Spin indicator={antIcon} />
    </div>
  );
}
