import { Injectable, NotFoundException } from '@nestjs/common';
import * as fs from 'fs';
import * as path from 'path';

export interface SyncMeta {
  last_modified: string;
  device_name: string;
}

export interface SyncData {
  db: string; // base64
  db_shm: string | null; // base64
  db_wal: string | null; // base64
  last_modified: string;
  device_name: string;
}

@Injectable()
export class SyncService {
  private readonly dataRoot: string;

  constructor() {
    this.dataRoot = process.env.DATA_DIR || path.join(process.cwd(), 'data');
  }

  private getUserDir(userId: string): string {
    return path.join(this.dataRoot, 'users', userId);
  }

  async upload(
    userId: string,
    files: {
      db?: Express.Multer.File[];
      db_shm?: Express.Multer.File[];
      db_wal?: Express.Multer.File[];
    },
    deviceName: string,
  ): Promise<void> {
    const userDir = this.getUserDir(userId);
    fs.mkdirSync(userDir, { recursive: true });

    if (files.db && files.db[0]) {
      fs.writeFileSync(path.join(userDir, 'database.db'), files.db[0].buffer);
    }
    if (files.db_shm && files.db_shm[0]) {
      fs.writeFileSync(
        path.join(userDir, 'database.db-shm'),
        files.db_shm[0].buffer,
      );
    }
    if (files.db_wal && files.db_wal[0]) {
      fs.writeFileSync(
        path.join(userDir, 'database.db-wal'),
        files.db_wal[0].buffer,
      );
    }

    const meta: SyncMeta = {
      last_modified: new Date().toISOString(),
      device_name: deviceName,
    };
    fs.writeFileSync(
      path.join(userDir, 'sync_meta.json'),
      JSON.stringify(meta),
    );
  }

  async download(userId: string): Promise<SyncData> {
    const userDir = this.getUserDir(userId);
    const dbPath = path.join(userDir, 'database.db');

    if (!fs.existsSync(dbPath)) {
      throw new NotFoundException('No config found for this user');
    }

    const meta = this.readMeta(userId);
    const db = fs.readFileSync(dbPath).toString('base64');

    const shmPath = path.join(userDir, 'database.db-shm');
    const walPath = path.join(userDir, 'database.db-wal');

    const db_shm = fs.existsSync(shmPath)
      ? fs.readFileSync(shmPath).toString('base64')
      : null;
    const db_wal = fs.existsSync(walPath)
      ? fs.readFileSync(walPath).toString('base64')
      : null;

    return {
      db,
      db_shm,
      db_wal,
      last_modified: meta.last_modified,
      device_name: meta.device_name,
    };
  }

  async getMeta(userId: string): Promise<SyncMeta> {
    return this.readMeta(userId);
  }

  private readMeta(userId: string): SyncMeta {
    const userDir = this.getUserDir(userId);
    const metaPath = path.join(userDir, 'sync_meta.json');

    if (!fs.existsSync(metaPath)) {
      throw new NotFoundException('No config found for this user');
    }

    return JSON.parse(fs.readFileSync(metaPath, 'utf-8')) as SyncMeta;
  }
}
