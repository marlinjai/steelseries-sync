import {
  Controller,
  Get,
  Put,
  Body,
  Req,
  UploadedFiles,
  UseInterceptors,
} from '@nestjs/common';
import { FileFieldsInterceptor } from '@nestjs/platform-express';
import { SyncService } from './sync.service';

@Controller('sync')
export class SyncController {
  constructor(private readonly syncService: SyncService) {}

  @Put()
  @UseInterceptors(
    FileFieldsInterceptor([
      { name: 'db', maxCount: 1 },
      { name: 'db_shm', maxCount: 1 },
      { name: 'db_wal', maxCount: 1 },
    ]),
  )
  async upload(
    @UploadedFiles()
    files: {
      db?: Express.Multer.File[];
      db_shm?: Express.Multer.File[];
      db_wal?: Express.Multer.File[];
    },
    @Body('device_name') deviceName: string,
    @Req() req: any,
  ) {
    const userId = req.user?.sub || 'anonymous';
    await this.syncService.upload(userId, files, deviceName || 'unknown');
    return { status: 'ok' };
  }

  @Get()
  async download(@Req() req: any) {
    const userId = req.user?.sub || 'anonymous';
    return this.syncService.download(userId);
  }

  @Get('meta')
  async meta(@Req() req: any) {
    const userId = req.user?.sub || 'anonymous';
    return this.syncService.getMeta(userId);
  }
}
