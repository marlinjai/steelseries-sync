import {
  Controller,
  Get,
  Put,
  Body,
  Req,
  UploadedFiles,
  UseGuards,
  UseInterceptors,
} from '@nestjs/common';
import { FileFieldsInterceptor } from '@nestjs/platform-express';
import { JwtAuthGuard } from '../auth/auth.guard';
import { SyncService } from './sync.service';

// Set by JwtStrategy.validate once the bearer token is verified.
interface AuthedRequest {
  user?: { sub?: string; email?: string };
}

@Controller('sync')
@UseGuards(JwtAuthGuard)
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
    @Req() req: AuthedRequest,
  ) {
    const userId = req.user?.sub || 'anonymous';
    await this.syncService.upload(userId, files, deviceName || 'unknown');
    return { status: 'ok' };
  }

  @Get()
  download(@Req() req: AuthedRequest) {
    const userId = req.user?.sub || 'anonymous';
    return this.syncService.download(userId);
  }

  @Get('meta')
  meta(@Req() req: AuthedRequest) {
    const userId = req.user?.sub || 'anonymous';
    return this.syncService.getMeta(userId);
  }
}
