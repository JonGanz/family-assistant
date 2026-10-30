UPDATE member SET color = 'blue' WHERE color IS NOT NULL;
ALTER TABLE member ADD CONSTRAINT member_color_hue
    CHECK (color IS NULL OR color IN ('red','orange','yellow','green','cyan','blue','purple','pink'));
