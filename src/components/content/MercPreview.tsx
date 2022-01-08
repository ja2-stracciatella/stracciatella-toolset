import { ExclamationOutlined } from "@ant-design/icons";
import { Avatar } from "antd";
import { useMemo } from "react";
import { useImageFile } from "../../hooks/useImage";

interface MercPreviewProps {
    profileId: number,
}

export function MercPreview({ profileId }: MercPreviewProps) {
    const graphic1 = useMemo(() => {
        return `faces/${profileId.toString().padStart(2, '0')}.sti`;
    }, [profileId]);
    const graphic2 = useMemo(() => {
        return `faces/B${profileId.toString().padStart(2, '0')}.sti`;
    }, [profileId]);
    const { data: image1, error: error1 } = useImageFile(graphic1);
    const { data: image2, error: error2 } = useImageFile(graphic2);
    const additionalAvatarProps = useMemo(() => {
        if (error1 && error2) {
            return { icon: <ExclamationOutlined /> }
        }
        if (!image1 && !image2) {
            return {};
        }
        return { src: image1 || image2 };
    }, [error1, error2, image1, image2]);

    return <Avatar shape="square" {...additionalAvatarProps} />;
}